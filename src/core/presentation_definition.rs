use super::credential_format::*;
use super::input_descriptor::*;
use super::presentation_submission::*;
use super::response::AuthorizationResponse;

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use ssi_claims::jwt::VerifiablePresentation;

/// A presentation definition is a JSON object that describes the information a [Verifier](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:verifier) requires of a [Holder](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:holder).
///
/// > Presentation Definitions are objects that articulate what proofs a [Verifier](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:verifier) requires.
/// > These help the [Verifier](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:verifier) to decide how or whether to interact with a [Holder](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:holder).
///
/// Presentation Definitions are composed of inputs, which describe the forms and details of the
/// proofs they require, and optional sets of selection rules, to allow [Holder](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:holder)s flexibility
/// in cases where different types of proofs may satisfy an input requirement.
///
/// For more information, see: [https://identity.foundation/presentation-exchange/spec/v2.0.0/#presentation-definition](https://identity.foundation/presentation-exchange/spec/v2.0.0/#presentation-definition)
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresentationDefinition {
    id: String,
    input_descriptors: Vec<InputDescriptor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<ClaimFormatMap>,
}

impl PresentationDefinition {
    /// The Presentation Definition MUST contain an id property. The value of this property MUST be a string.
    /// The string SHOULD provide a unique ID for the desired context.
    ///
    /// The Presentation Definition MUST contain an input_descriptors property. Its value MUST be an array of Input Descriptor Objects,
    /// the composition of which are found [InputDescriptor] type.
    ///
    pub fn new(id: String, input_descriptor: InputDescriptor) -> Self {
        Self {
            id,
            input_descriptors: vec![input_descriptor],
            ..Default::default()
        }
    }

    /// Return the id of the presentation definition.
    pub fn id(&self) -> &String {
        &self.id
    }

    /// Add a new input descriptor to the presentation definition.
    pub fn add_input_descriptors(mut self, input_descriptor: InputDescriptor) -> Self {
        self.input_descriptors.push(input_descriptor);
        self
    }

    /// Return the input descriptors of the presentation definition.
    pub fn input_descriptors(&self) -> &Vec<InputDescriptor> {
        &self.input_descriptors
    }

    /// Return a mutable reference to the input descriptors of the presentation definition.
    pub fn input_descriptors_mut(&mut self) -> &mut Vec<InputDescriptor> {
        &mut self.input_descriptors
    }

    /// Set the name of the presentation definition.
    ///
    /// The [PresentationDefinition] MAY contain a name property. If present, its value SHOULD be a
    /// human-friendly string intended to constitute a distinctive designation of the Presentation Definition.
    pub fn set_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Return the name of the presentation definition.
    pub fn name(&self) -> Option<&String> {
        self.name.as_ref()
    }

    /// Set the purpose of the presentation definition.
    ///
    /// The [PresentationDefinition] MAY contain a purpose property. If present, its value MUST be a string that
    /// describes the purpose for which the Presentation Definition's inputs are being used for.
    pub fn set_purpose(mut self, purpose: String) -> Self {
        self.purpose = Some(purpose);
        self
    }

    /// Return the purpose of the presentation definition.
    pub fn purpose(&self) -> Option<&String> {
        self.purpose.as_ref()
    }

    /// Attach a format to the presentation definition.
    ///
    /// The Presentation Definition MAY include a format property. If present,
    /// the value MUST be an object with one or more properties matching the
    /// registered Claim Format Designations (e.g., jwt, jwt_vc, jwt_vp, etc.).
    ///
    /// The properties inform the [Holder](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:holder) of the Claim format configurations the [Verifier](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:verifier) can process.
    /// The value for each claim format property MUST be an object composed as follows:
    ///
    /// The object MUST include a format-specific property (i.e., alg, proof_type)
    /// that expresses which algorithms the [Verifier](https://identity.foundation/presentation-exchange/spec/v2.0.0/#term:verifier) supports for the format.
    /// Its value MUST be an array of one or more format-specific algorithmic identifier references,
    /// as noted in the Claim Format Designations section.
    ///
    /// See: [https://identity.foundation/presentation-exchange/spec/v2.0.0/#presentation-definition](https://identity.foundation/presentation-exchange/spec/v2.0.0/#presentation-definition)
    pub fn set_format(mut self, format: ClaimFormatMap) -> Self {
        self.format = Some(format);
        self
    }

    /// Validate a presentation submission against the presentation definition.
    ///
    /// Checks the underlying presentation submission parsed from the authorization response,
    /// against the input descriptors of the presentation definition.
    pub async fn validate_authorization_response(
        &self,
        auth_response: &AuthorizationResponse,
    ) -> Result<()> {
        match auth_response {
            AuthorizationResponse::Jwt(_jwt) => {
                // TODO: Handle JWT Encoded authorization response.

                bail!("Authorization Response Presentation Definition Validation Not Implemented.")
            }
            AuthorizationResponse::Unencoded(response) => {
                let presentation_submission = response.presentation_submission().parsed();

                let jwt = response.vp_token().0.clone();

                let verifiable_presentation: VerifiablePresentation =
                    ssi_claims::jwt::decode_unverified(&jwt)?;

                // Ensure the definition id matches the submission's definition id.
                if presentation_submission.definition_id() != self.id() {
                    bail!("Presentation Definition ID does not match the Presentation Submission.")
                }

                let descriptor_map: HashMap<String, DescriptorMap> = presentation_submission
                    .descriptor_map()
                    .iter()
                    .map(|descriptor_map| (descriptor_map.id().to_owned(), descriptor_map.clone()))
                    .collect();

                for input_descriptor in self.input_descriptors().iter() {
                    match descriptor_map.get(input_descriptor.id()) {
                        None => {
                            // TODO: Determine whether input descriptor must have a corresponding descriptor map.
                            bail!("Input Descriptor ID not found in Descriptor Map.")
                        }
                        Some(descriptor) => {
                            input_descriptor
                                .validate_verifiable_presentation(
                                    &verifiable_presentation,
                                    descriptor,
                                )
                                .context("Input Descriptor Validation Failed.")?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Add a new format to the presentation definition.
    pub fn add_format(mut self, format: ClaimFormatDesignation, value: ClaimFormatPayload) -> Self {
        self.format
            .get_or_insert_with(HashMap::new)
            .insert(format, value);
        self
    }

    /// Return the format of the presentation definition.
    pub fn format(&self) -> Option<&ClaimFormatMap> {
        self.format.as_ref()
    }
}