use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    String,
    Number,
    Integer,
    Boolean,
    Array,
    Object,
}

/// Schema Validator is a JSON Schema descriptor used to evaluate the return value of a JsonPath
/// expression, used by the presentation definition constraints field to ensure the property value
/// meets the expected schema.
///
/// For more information, see the field constraints filter property:
///
/// - [https://identity.foundation/presentation-exchange/spec/v2.0.0/#input-descriptor-object](https://identity.foundation/presentation-exchange/spec/v2.0.0/#input-descriptor-object)
///
/// - [https://json-schema.org/understanding-json-schema](https://json-schema.org/understanding-json-schema)
///
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SchemaValidator {
    #[serde(rename = "type")]
    schema_type: SchemaType,
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    min_length: Option<usize>,
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    maximum: Option<f64>,
    #[serde(rename = "exclusiveMinimum", skip_serializing_if = "Option::is_none")]
    exclusive_minimum: Option<f64>,
    #[serde(rename = "exclusiveMaximum", skip_serializing_if = "Option::is_none")]
    exclusive_maximum: Option<f64>,
    #[serde(rename = "multipleOf", skip_serializing_if = "Option::is_none")]
    multiple_of: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    required: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    properties: HashMap<String, Box<SchemaValidator>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Box<SchemaValidator>>,
}

impl PartialEq for SchemaValidator {
    fn eq(&self, other: &Self) -> bool {
        self.schema_type == other.schema_type
            && self.min_length == other.min_length
            && self.max_length == other.max_length
            && self.pattern == other.pattern
            && self.minimum == other.minimum
            && self.maximum == other.maximum
            && self.required == other.required
            && self.properties == other.properties
            && self.items == other.items
    }
}

impl Eq for SchemaValidator {}

impl SchemaValidator {
    /// Creates a new schema validator with the given schema type.
    pub fn new(schema_type: SchemaType) -> Self {
        Self {
            schema_type,
            min_length: None,
            max_length: None,
            pattern: None,
            minimum: None,
            maximum: None,
            exclusive_minimum: None,
            exclusive_maximum: None,
            multiple_of: None,
            required: Vec::new(),
            properties: HashMap::new(),
            items: None,
        }
    }

    pub fn set_schema_type(mut self, schema_type: SchemaType) -> Self {
        self.schema_type = schema_type;
        self
    }

    pub fn set_min_length(mut self, min_length: usize) -> Self {
        self.min_length = Some(min_length);
        self
    }

    pub fn set_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    pub fn set_pattern(mut self, pattern: String) -> Self {
        self.pattern = Some(pattern);
        self
    }

    pub fn set_minimum(mut self, minimum: f64) -> Self {
        self.minimum = Some(minimum);
        self
    }

    pub fn set_maximum(mut self, maximum: f64) -> Self {
        self.maximum = Some(maximum);
        self
    }

    pub fn set_exclusive_minimum(mut self, exclusive_minimum: f64) -> Self {
        self.exclusive_minimum = Some(exclusive_minimum);
        self
    }

    pub fn set_exclusive_maximum(mut self, exclusive_maximum: f64) -> Self {
        self.exclusive_maximum = Some(exclusive_maximum);
        self
    }

    pub fn set_multiple_of(mut self, multiple_of: f64) -> Self {
        self.multiple_of = Some(multiple_of);
        self
    }

    pub fn add_required(mut self, required: String) -> Self {
        self.required.push(required);
        self
    }

    pub fn add_property(mut self, key: String, value: SchemaValidator) -> Self {
        self.properties.insert(key, Box::new(value));
        self
    }

    pub fn set_items(mut self, items: Box<SchemaValidator>) -> Self {
        self.items = Some(items);
        self
    }

    pub fn validate(&self, value: &Value) -> Result<()> {
        match self.schema_type {
            SchemaType::String => self.validate_string(value),
            SchemaType::Number => self.validate_number(value),
            SchemaType::Integer => self.validate_integer(value),
            SchemaType::Boolean => self.validate_boolean(value),
            SchemaType::Array => self.validate_array(value),
            SchemaType::Object => self.validate_object(value),
        }
    }

    pub fn validate_string(&self, value: &Value) -> Result<()> {
        let s = value.as_str().context("Expected a string")?;

        if let Some(min_length) = self.min_length {
            if s.len() <= min_length {
                bail!(
                    "String length {} is less than minimum {}",
                    s.len(),
                    min_length
                );
            }
        }

        if let Some(max_length) = self.max_length {
            if s.len() >= max_length {
                bail!(
                    "String length {} is greater than maximum {}",
                    s.len(),
                    max_length
                );
            }
        }

        if let Some(pattern) = &self.pattern {
            let regex_pattern = Regex::new(pattern).context("Invalid regex pattern")?;

            if !regex_pattern.is_match(pattern) {
                bail!("String does not match pattern: {}", pattern);
            }
        }

        Ok(())
    }

    pub fn validate_number(&self, value: &Value) -> Result<()> {
        let n = value.as_f64().context("Expected a number")?;

        if let Some(minimum) = self.minimum {
            if n <= minimum {
                bail!("Number {} is less than minimum {}", n, minimum);
            }
        }

        if let Some(maximum) = self.maximum {
            if n >= maximum {
                bail!("Number {} is greater than maximum {}", n, maximum);
            }
        }

        if let Some(exclusive_minimum) = self.exclusive_minimum {
            if n < exclusive_minimum {
                bail!(
                    "Number {} is less than or equal to exclusive minimum {}",
                    n,
                    exclusive_minimum
                );
            }
        }

        if let Some(exclusive_maximum) = self.exclusive_maximum {
            if n > exclusive_maximum {
                bail!(
                    "Number {} is greater than or equal to exclusive maximum {}",
                    n,
                    exclusive_maximum
                );
            }
        }

        if let Some(multiple_of) = self.multiple_of {
            if n % multiple_of != 0.0 {
                bail!("Number {} is not a multiple of {}", n, multiple_of);
            }
        }

        Ok(())
    }

    pub fn validate_integer(&self, value: &Value) -> Result<()> {
        let n = value.as_i64().context("Expected an integer")?;

        if let Some(minimum) = self.minimum {
            if n <= minimum as i64 {
                bail!("Integer {} is less than minimum {}", n, minimum);
            }
        }

        if let Some(maximum) = self.maximum {
            if n >= maximum as i64 {
                bail!("Integer {} is greater than maximum {}", n, maximum);
            }
        }

        if let Some(exclusive_minimum) = self.exclusive_minimum {
            if n < exclusive_minimum as i64 {
                bail!(
                    "Integer {} is less than or equal to exclusive minimum {}",
                    n,
                    exclusive_minimum
                );
            }
        }

        if let Some(exclusive_maximum) = self.exclusive_maximum {
            if n > exclusive_maximum as i64 {
                bail!(
                    "Integer {} is greater than or equal to exclusive maximum {}",
                    n,
                    exclusive_maximum
                );
            }
        }

        if let Some(multiple_of) = self.multiple_of {
            if n % multiple_of as i64 != 0 {
                bail!("Integer {} is not a multiple of {}", n, multiple_of);
            }
        }

        Ok(())
    }

    pub fn validate_boolean(&self, value: &Value) -> Result<()> {
        if !value.is_boolean() {
            bail!("Expected a boolean".to_string());
        }
        Ok(())
    }

    pub fn validate_array(&self, value: &Value) -> Result<()> {
        let arr = value.as_array().context("Expected an array")?;

        if let Some(min_length) = self.min_length {
            if arr.len() < min_length {
                bail!(
                    "Array length {} is less than minimum {}",
                    arr.len(),
                    min_length
                );
            }
        }

        if let Some(max_length) = self.max_length {
            if arr.len() > max_length {
                bail!(
                    "Array length {} is greater than maximum {}",
                    arr.len(),
                    max_length
                );
            }
        }

        if let Some(item_validator) = &self.items {
            for (index, item) in arr.iter().enumerate() {
                item_validator
                    .validate(item)
                    .context(format!("Error in array item {}", index))?;
            }
        }

        Ok(())
    }

    pub fn validate_object(&self, value: &Value) -> Result<()> {
        let obj = value.as_object().context("Expected an object")?;

        for required_prop in &self.required {
            if !obj.contains_key(required_prop) {
                bail!("Missing required property: {}", required_prop);
            }
        }

        for (prop_name, prop_validator) in &self.properties {
            if let Some(prop_value) = obj.get(prop_name) {
                prop_validator
                    .validate(prop_value)
                    .context(format!("Error in property {}", prop_name))?;
            }
        }

        Ok(())
    }
}
