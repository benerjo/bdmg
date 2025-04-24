/*
    Copyright 2020 benerjo

    This file is part of bdmg2k.

    bdmg is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    bdmg is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with bdmg.  If not, see <https://www.gnu.org/licenses/>
*/

///Enumeration used to specify the type of an attribute
/// in case of a reference, the name of the referenced attribute
/// is given as parameter.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum BaseAttributeType {
    Integer,
    String,
    Reference(String),
}

///Enumeration to specify the kind of attribute: mandatory or
/// optional
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum AttributeType {
    Mandatory(BaseAttributeType),
    Optional(BaseAttributeType),
}

impl AttributeType {
    ///Retrieve the base type
    pub fn get_base_type(&self) -> &BaseAttributeType {
        match self {
            AttributeType::Mandatory(base) => base,
            AttributeType::Optional(base) => base,
        }
    }
}

///The definition of an attribute
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Attribute {
    /// The name of the attribute
    name: String,
    /// The type of the attribute
    is: AttributeType,
    /// Optional, comments relative to the attribute
    comm: Option<String>,
    /// Optional, whether the attribute is mutable or not. Default is false
    mutable: Option<bool>,
    /// Optional, whether the attribute could be used as index in queries.
    /// If set to true, the system expect the value to be unique in the column
    indexable: Option<bool>,
    /// Optional, whether the attribute should be considered as secret
    /// a secret attribute will not be deserialized
    secret: Option<bool>,
}

impl Attribute {
    /// Retrieve the name of the attribute
    pub fn get_name(&self) -> &String {
        &self.name
    }

    /// Retrieve the comment describing this attribute
    pub fn get_comment(&self) -> &Option<String> {
        &self.comm
    }

    /// Retrieve the type of the attribute
    pub fn get_type(&self) -> &AttributeType {
        &self.is
    }

    /// Check if the attribute is secret. A secret attribute
    /// should not be deserialized
    pub fn is_secret(&self) -> bool {
        self.secret.unwrap_or(false)
    }

    /// Check if the attribute can be used as index in a query
    pub fn is_indexable(&self) -> bool {
        self.indexable.unwrap_or(false)
    }

    /// Check if the attribute is optional
    pub fn is_optional(&self) -> bool {
        match &self.is {
            AttributeType::Optional(_) => true,
            AttributeType::Mandatory(_) => false,
        }
    }

    /// Retrieve the type of the referenced attribute, if any
    pub fn get_reference(&self) -> Option<&String> {
        match &self.is.get_base_type() {
            BaseAttributeType::Reference(r) => Some(r),
            _ => None,
        }
    }

    /// Check if the attribute is mutable
    pub fn is_mutable(&self) -> bool {
        self.mutable.unwrap_or(false)
    }
}
