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

use crate::attributes::{Attribute, AttributeType, BaseAttributeType};

use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub struct Object {
    /// The name of the table that contains the instances on database
    tbnm: String,
    /// The name of the object type
    name: String,
    /// The array of attribute of this object
    attr: Vec<Attribute>,
    /// The comments related to this object
    comm: Option<String>,
    /// The category in which the object should be set. This category name
    /// does not prevent name clashes if two objects have the same name
    category: Option<String>,
    /// A reference to a function that will check the validity of the instance.
    /// This validity check will be performed whenever an object is created
    /// or when a value is changed.
    /// Note that this will not be executed when retrieving an object from
    /// database. Therefore, a loaded instance may not pass this check
    /// if something went wrong in the database or if a user made change
    /// to the database directly
    validator: Option<String>,
    /// The list of objects that are referencing this object
    #[serde(skip)]
    referencing: Vec<String>,
}

impl Object {
    ///Retrieve the name of the object as defined in the object store
    pub fn get_name(&self) -> &String {
        &self.name
    }

    ///Retrieve the name of the database table that contain the object
    pub fn get_table_name(&self) -> &String {
        &self.tbnm
    }

    //Check if there is a validator installed on this object
    pub fn get_validator(&self) -> &Option<String> {
        &self.validator
    }

    ///Retrieve the category in which this object is defined
    pub fn get_category(&self) -> &Option<String> {
        &self.category
    }

    ///Retrieve the description of the object
    pub fn get_description(&self) -> &Option<String> {
        &self.comm
    }

    ///Retrieve an iterator to the attributes of this object
    pub fn get_attributes(&self) -> std::slice::Iter<'_, Attribute> {
        self.attr.iter()
    }

    ///Check if the object has attributes, private or not
    pub fn has_attributes(&self) -> bool {
        !self.attr.is_empty()
    }

    ///Check if the object has public attribues. A public attribute
    /// is an attribute that is not secret.
    pub fn has_public_attributes(&self) -> bool {
        for at in &self.attr {
            if !at.is_secret() {
                return true;
            }
        }
        return false;
    }

    /// Retrieve the attribute that is referencing the object whose name is the
    /// given parameter
    pub fn get_relation_attribute(&self, referenced: &str) -> Option<&Attribute> {
        for at in self.get_attributes() {
            match at.get_reference() {
                Some(r) => {
                    if r == referenced {
                        return Some(at);
                    }
                }
                None => {}
            }
        }
        None
    }

    ///Check if the object has any relation defined in this object.
    ///Note: only the object containing a reference to some object
    ///will return true. Not the object being referenced.
    pub fn has_relations(&self) -> bool {
        for at in self.get_attributes() {
            match at.get_reference() {
                Some(_referenced) => return true,
                None => {}
            }
        }
        false
    }

    ///Add the knowledge that this object is being referenced in some
    /// object having the given name
    pub fn add_referencing_object(&mut self, referencing: String) {
        self.referencing.push(referencing)
    }

    ///Check if the object is being referenced in any other object
    pub fn is_referenced(&self) -> bool {
        self.referencing.len() > 0
    }

    ///Retrieve the list of object names referencing this object
    pub fn get_referencing_objects(&self) -> std::slice::Iter<'_, String> {
        self.referencing.iter()
    }

    /// Check if this object represent a n-to-n relation between
    /// two objects, and the content of an instance can't be changed
    /// In practice, this measn that the object contains only 2 data:
    /// both of them mandatory immutable references
    pub fn is_object_immutable_relation(&self) -> Option<(&Attribute, &Attribute)> {
        let mut from = None;
        let mut to = None;
        for at in self.get_attributes() {
            if at.is_optional() || at.is_mutable() {
                return None;
            }
            match at.get_reference() {
                Some(_refered) => {
                    if from.is_none() {
                        from = Some(at);
                    } else if to.is_none() {
                        to = Some(at);
                    } else {
                        return None;
                    }
                }
                None => return None,
            }
        }
        if from.is_some() && to.is_some() {
            return Some((from.unwrap(), to.unwrap()));
        } else {
            return None;
        }
    }

    /// Check if this object represent a n-to-n relation between
    /// two objects.
    /// In practice, this means that the object contains only 2
    /// data: both of them mandatory references
    pub fn is_object_relation(&self) -> Option<(&String, &String)> {
        let mut from = None;
        let mut to = None;
        for at in self.get_attributes() {
            if at.is_optional() {
                return None;
            }
            match at.get_reference() {
                Some(refered) => {
                    if from.is_none() {
                        from = Some(refered);
                    } else if to.is_none() {
                        to = Some(refered);
                    } else {
                        return None;
                    }
                }
                None => return None,
            }
        }
        if from.is_some() && to.is_some() {
            return Some((from.unwrap(), to.unwrap()));
        } else {
            return None;
        }
    }

    ///Check that every refenced object has a definition
    ///Parmeter: the map containg all object indexed by their name
    pub fn validate<'a, 'b, 'c>(
        &self,
        objects_map: &'c HashMap<&'a String, &'b Object>,
    ) -> Result<(), String> {
        for at in self.get_attributes() {
            let reference = match at.get_type() {
                AttributeType::Mandatory(BaseAttributeType::Reference(r)) => Some(r),
                AttributeType::Optional(BaseAttributeType::Reference(r)) => Some(r),
                _ => None,
            };
            match reference {
                Some(r) => {
                    if !objects_map.contains_key(r) {
                        return Err(format!(
                            "Unknown referenced type '{ref_type_name}' in '{object_name}.{attribute_name}'",
                            ref_type_name = r,
                            object_name = self.get_name(),
                            attribute_name = at.get_name()
                        ));
                    }
                }
                None => {}
            }
        }
        Ok(())
    }
}
