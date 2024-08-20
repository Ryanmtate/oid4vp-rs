use oid4vp::presentation_exchange::*;

use oid4vp::{
    core::{
        authorization_request::parameters::{ClientMetadata, Nonce, ResponseMode, ResponseType},
        object::UntypedObject,
        response::{parameters::VpToken, AuthorizationResponse, UnencodedAuthorizationResponse},
    },
    presentation_exchange::{PresentationDefinition, PresentationSubmission},
    verifier::session::{Outcome, Status},
    wallet::Wallet,
};
use ssi_jwk::Algorithm;

mod jwt_vc;
mod jwt_vp;

#[tokio::test]
async fn w3c_vc_did_client_direct_post() {
    let (wallet, verifier) = jwt_vc::wallet_verifier().await;

    let presentation_definition = PresentationDefinition::new(
        "did-key-id-proof".into(),
        InputDescriptor::new(
            "did-key-id".into(),
            Constraints::new()
                .add_constraint(
                    ConstraintsField::new(
                        "$.vp.verifiableCredential[0].vc.credentialSubject.id".into(),
                    )
                    .set_name("Verify Identity Key".into())
                    .set_purpose("Check whether your identity key has been verified.".into())
                    .set_filter(serde_json::json!({
                        "type": "string",
                        "pattern": "did:key:.*"
                    }))
                    .set_predicate(Predicate::Required),
                )
                .set_limit_disclosure(ConstraintsLimitDisclosure::Required),
        )
        .set_name("DID Key Identity Verification".into())
        .set_purpose("Check whether your identity key has been verified.".into())
        .set_format((|| {
            let mut map = ClaimFormatMap::new();
            map.insert(
                ClaimFormatDesignation::JwtVp,
                ClaimFormatPayload::Alg(vec![Algorithm::ES256.to_string()]),
            );
            map
        })()),
    );

    let client_metadata = UntypedObject::default();

    #[cfg(feature = "rand")]
    let nonce = Nonce::random(&mut rand::thread_rng());

    #[cfg(not(feature = "rand"))]
    let nonce = Nonce::from("random_nonce");

    let (id, request) = verifier
        .build_authorization_request()
        .with_presentation_definition(presentation_definition.clone())
        .with_request_parameter(ResponseMode::DirectPost)
        .with_request_parameter(ResponseType::VpToken)
        .with_request_parameter(nonce)
        .with_request_parameter(ClientMetadata(client_metadata))
        .build(wallet.metadata().clone())
        .await
        .unwrap();

    println!("Request: {:?}", request);

    let request = wallet.validate_request(request).await.unwrap();

    let parsed_presentation_definition = request
        .resolve_presentation_definition(wallet.http_client())
        .await
        .unwrap();

    assert_eq!(
        &presentation_definition,
        parsed_presentation_definition.parsed()
    );

    assert_eq!(&ResponseType::VpToken, request.response_type());

    assert_eq!(&ResponseMode::DirectPost, request.response_mode());

    let descriptor_map = parsed_presentation_definition
        .parsed()
        .input_descriptors()
        .iter()
        .map(|descriptor| {
            // NOTE: the input descriptor constraint field path is relative to the path
            // of the descriptor map matching the input descriptor id.
            DescriptorMap::new(
                descriptor.id().to_string(),
                // NOTE: Since the input descriptor may support several different claim format types. This value should not be
                // hardcoded in production code, but should be selected from available formats in the presentation definition
                // input descriptor.
                //
                // In practice, this format will be determined by the VDC collection's credential format.
                ClaimFormatDesignation::JwtVc,
                "$".into(),
            )
        })
        .collect();

    let presentation_submission = PresentationSubmission::new(
        uuid::Uuid::new_v4(),
        parsed_presentation_definition.parsed().id().clone(),
        descriptor_map,
    );

    let response = AuthorizationResponse::Unencoded(UnencodedAuthorizationResponse(
        Default::default(),
        VpToken(include_str!("examples/vp.jwt").to_owned()),
        presentation_submission.try_into().unwrap(),
    ));

    let status = verifier.poll_status(id).await.unwrap();
    assert_eq!(Status::SentRequest, status);

    let redirect = wallet.submit_response(request, response).await.unwrap();

    assert_eq!(None, redirect);

    let status = verifier.poll_status(id).await.unwrap();
    match status {
        Status::Complete(Outcome::Success { .. }) => (),
        _ => panic!("unexpected status: {status:?}"),
    }
}
