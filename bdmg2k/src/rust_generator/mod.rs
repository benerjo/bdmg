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

mod free_fn;
mod rust_impl;
mod traits_impl;

use crate::object::Object;
use crate::objectdb::{ObjectDB, RustOutputType};
use crate::{Attribute, AttributeType, BaseAttributeType, Error};

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use self::free_fn::generate_rust_free_functions;
use self::rust_impl::generate_rust_impl;
use self::traits_impl::generate_traits_impl;

fn get_lib_file(mut destination: PathBuf) -> Result<File, Error> {
    if !destination.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: match destination.to_str() {
                Some(p) => String::from(p),
                None => String::from("UNKNOWN"),
            },
        });
    }

    destination.push("lib");
    destination.set_extension("rs");

    match File::create(destination.as_path()) {
        Err(_e) => {
            return Err(Error::UnableToCreateFile {
                file: match destination.as_path().to_str() {
                    Some(pth) => String::from(pth),
                    None => String::from("lib.rs"),
                },
            })
        }
        Ok(f) => Ok(f),
    }
}

///Retrieve the file that should contain the mod file
///
/// The file will be located inside of the destination that is given as parameter
fn get_mod_file(mut destination: PathBuf) -> Result<File, Error> {
    if !destination.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: match destination.to_str() {
                Some(p) => String::from(p),
                None => String::from("UNKNOWN"),
            },
        });
    }

    destination.push("mod");
    destination.set_extension("rs");

    match File::create(destination.as_path()) {
        Err(_e) => {
            return Err(Error::UnableToCreateFile {
                file: match destination.as_path().to_str() {
                    Some(pth) => String::from(pth),
                    None => String::from("mod.rs"),
                },
            })
        }
        Ok(f) => Ok(f),
    }
}

///Retrieve the file that will contain the rust code related to the given object
///
/// # Error
/// An error will be returned in the following cases:
/// - the path is not a directory
/// - we were not able to create the corresponding file
fn get_object_file<'a>(object: &Object, path: &'a Path) -> Result<File, Error> {
    if !path.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: match path.to_str() {
                Some(p) => String::from(p),
                None => String::from("UNKNOWN"),
            },
        });
    }
    let mut pbuf = PathBuf::from(path);
    let lowercase = object.get_name().to_ascii_lowercase();
    pbuf.push(&lowercase);
    pbuf.set_extension("rs");

    match File::create(pbuf.as_path()) {
        Err(_e) => {
            return Err(Error::UnableToCreateFile {
                file: match pbuf.to_str() {
                    Some(pth) => String::from(pth),
                    None => format!("{}.rs", lowercase),
                },
            })
        }
        Ok(f) => Ok(f),
    }
}

fn get_lib_file_content(objects: &ObjectDB) -> Result<String, Error> {
    let content = format!(
        "{dependencies}\n\n{mod_content}",
        dependencies =
            "#[macro_use]\nextern crate diesel;\n#[macro_use]\nextern crate serde_derive;",
        mod_content = get_mod_file_content(objects)?
    );
    Ok(content)
}

/// Generate the content of the mod file.
///
/// This should contain the pub use of all objects and the documentation related
/// to the module.
fn get_mod_file_content(objects: &ObjectDB) -> Result<String, Error> {
    let mut usings = String::new();
    for obj in objects.get_objects() {
        usings = usings
            + &format!(
                "mod {module_name};\npub use {module_name}::{object_name};\npub use {module_name}::Id{object_name};\n",
                module_name = obj.get_name().to_ascii_lowercase(),
                object_name = obj.get_name()
            );
    }
    usings += "pub mod schema;\n";

    let mut inserts = String::new();
    for obj in objects.get_objects() {
        inserts = inserts
            + &format!(
                "
    objects.insert(String::from(\"{object_name}\"), {{
        let obj_intro = {object_module}::{object_name}::get_object_introspection();
        let mut attr_vec = obj_intro.get_attributes();
        let mut attributes = std::collections::BTreeMap::new();
        for at in attr_vec.drain(..) {{
           attributes.insert(at.get_name().clone(), at);
        }}
        (obj_intro, attributes)
    }});",
                object_name = obj.get_name(),
                object_module = obj.get_name().to_ascii_lowercase()
            );
    }

    let content = &format!(
        "
use bdmg::{{ObjectIntrospection, SqlRepresentation}};

///Retreive a map that links the different object names to a pair containing a) the ObjectIntrospection
/// related to the object and b) the map describing the different attributes
pub fn get_objects() -> std::collections::BTreeMap<String, (Box<dyn ObjectIntrospection>, std::collections::BTreeMap<String, bdmg::Attribute>)> {{
    let mut objects = std::collections::BTreeMap::new();{inserts}
    objects
}}
");
    Ok(format!("{usings}{content}\n"))
}

/// Retrieve the rust representation of the object
fn get_object_mod_file_content(object: &Object, db: &ObjectDB, path: &Path) -> Result<(), Error> {
    match generate_rust(object, db, path) {
        Err(_e) => {
            return Err(Error::UnableToWriteCodeForObject {
                object_name: object.get_name().clone(),
            })
        }
        Ok(_) => {}
    }

    return Ok(());
}

///Generate the code needed to be able to use the different objects defined on a database through a rust interface.
///
/// # Error
/// Errors will be generated in one of the following case is met:
/// - a file can not be opened
/// - the destination is not a directory
/// - the full content has not been written
pub fn generate_code(
    objects: &ObjectDB,
    destination: &str,
    output_type: RustOutputType,
) -> Result<(), Error> {
    let pbuf = PathBuf::from(destination);

    if !pbuf.exists() {
        match std::fs::create_dir(&pbuf) {
            Ok(()) => {},
            Err(e) => return Err(Error::UnableToCreateOutputDirectory { destination: destination.to_string(), error: e }),
        }
    }

    if !pbuf.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: destination.to_string(),
        });
    }

    //first, let us generate the different rust struct
    let path = pbuf.as_path();
    for object in objects.get_objects() {
        get_object_mod_file_content(object, objects, path)?;
    }

    //second, let us generate the mod file to have all object structs public
    let (file_content, mut file) = if output_type == RustOutputType::Module {
        (get_mod_file_content(objects)?, get_mod_file(pbuf)?)
    } else {
        (get_lib_file_content(objects)?, get_lib_file(pbuf)?)
    };
    match file.write(file_content.as_bytes()) {
        Err(_e) => Err(Error::UnableToWriteToFile {
            file: file,
            content: file_content,
        }),
        Ok(size_written) => {
            if size_written == file_content.as_bytes().len() {
                Ok(())
            } else {
                Err(Error::UnableToWriteToFile {
                    file: file,
                    content: file_content,
                })
            }
        }
    }
}

///Generate the rust code to represent the object as a rust object whose data
///is linked to a SQLite database
fn generate_rust<'a>(object: &Object, db: &ObjectDB, path: &'a Path) -> Result<(), Error> {
    let file_content = format!(
        "{}\n{}\n{}\n{}\n{}",
        generate_rust_include(object),
        generate_rust_struct(object),
        generate_traits_impl(object, db),
        generate_rust_impl(object, db),
        generate_rust_free_functions(object)
    );

    let mut file = get_object_file(object, path)?;

    match file.write(file_content.as_bytes()) {
        Err(_e) => Err(Error::UnableToWriteToFile {
            file: file,
            content: file_content,
        }),
        Ok(_) => Ok(()),
    }
}

fn generate_rust_include(object: &Object) -> String {
    //First let us construct the unique list of referenced types
    let mut refs = Vec::new();
    for at in object.get_attributes() {
        match at.get_reference() {
            Some(r) => {
                match refs.iter().find(|&x| x == &r) {
                    Some(_) => {}
                    None => refs.push(r),
                };
            }
            None => {}
        }
    }

    //Create the extra imports to referenced object types
    let mut extra_imports = String::new();
    for nm in refs {
        extra_imports = extra_imports
            + &format!(
                "\nuse super::{ObjectName};\nuse super::{module}::Id{ObjectName};",
                ObjectName = nm,
                module = nm.to_lowercase()
            );
    }

    //Generate the list of imports to other object types, to the used library and to the schema
    format!(
        "use bdmg::{{SqlRepresentation,Object,ObjectIntrospection,ObjectIterator,ObjectFactory}};
{imports}
use super::schema::{table_name};

use diesel;
use diesel::prelude::*;
",
        imports = extra_imports,
        table_name = object.get_table_name()
    )
}

/// Generate the rust struct representing the object
/// it supposes that the struct will be defined at the top level indentation
fn generate_rust_struct(object: &Object) -> String {
    //the derive macro for the struct. If it has relations, we must use the Associations derive of Diesel
    let derive_macros = if object.has_relations() {
        let mut belongs = String::new();
        for at in object.get_attributes() {
            match at.get_reference() {
                Some(r) => {
                    belongs = belongs
                        + &format!(
                            "#[diesel(belongs_to({}, foreign_key = {}))]\n",
                            r,
                            at.get_name()
                        );
                }
                None => {}
            }
        }
        format!("#[derive(Queryable, Insertable, Serialize, Deserialize, Associations, Clone, Debug)]\n{belong}", belong = belongs)
    } else {
        String::from("#[derive(Queryable, Insertable, Serialize, Deserialize, Clone, Debug)]")
    };

    let mut atdef = String::new();
    let mut attribute_conversion = String::new();
    for at in object.get_attributes() {
        match at.get_comment() {
            Some(comm) => {
                atdef = atdef
                    + &format!(
                        "    /// {}\n",
                        comm.replace("\r\n", "\n").replace("\n", "\n    /// "),
                    )
            }
            None => {}
        };
        if at.is_secret() {
            atdef = atdef + &format!("#[serde(skip_serializing)]\n");
        };
        atdef = atdef + "    " + at.get_name() + ": " + &get_rust_type(at) + ",\n";
        attribute_conversion = format!(
            "{attribute_conversion}{attribute_name}: value.{attribute_name},\n            ",
            attribute_name = at.get_name()
        );
    }

    let comment = match object.get_description() {
        Some(comments) => comments.replace("\r\n", "\n").replace("\n", "\n/// "),
        None => String::new(),
    };

    let id_struct = format!(
        "/// The structure containing the id of {object_name}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Id{object_name} {{
    pub(crate) id: i32
}}

///Structure like {object_name}, without the id.
/// Used internally when creating new {object_name} instances
#[derive(Insertable, Serialize)]
#[diesel(table_name = {table_name})]
struct Insertable{object_name} {{
    {attributes}
    version: i64,
}}

impl From<{object_name}> for Insertable{object_name} {{
    fn from(value: {object_name}) -> Self {{
        Self {{
            {attribute_conversion}version: value.version,
        }}
    }}
}}

impl From<(i32, Insertable{object_name})> for {object_name} {{
    fn from((new_id, value): (i32, Insertable{object_name})) -> Self {{
        Self {{
            id: new_id,
            {attribute_conversion}version: value.version,
        }}
    }}
}}

impl<DB> diesel::deserialize::FromSql<diesel::sql_types::Integer, DB> for Id{object_name}
where
    DB: diesel::backend::Backend,
    i32: diesel::deserialize::FromSql<diesel::sql_types::Integer, DB>,
{{
    fn from_sql(
        bytes: <DB as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {{
        Ok(Self {{
            id: i32::from_sql(bytes)?,
        }})
    }}
}}

impl std::fmt::Display for Id{object_name} {{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
        write!(f, \"{{}}\", self.id)
    }}
}}

impl<'de> serde::de::Deserialize<'de> for Id{object_name} {{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {{
        let v = deserializer.deserialize_i32(bdmg::ObjectIdVisitor {{}})? as i32;
        Ok(Id{object_name} {{ id: v }})
    }}
}}
",
        object_name = object.get_name(),
        attributes = atdef,
        table_name = object.get_table_name(),
    );

    format!(
        "/// {struct_comments}\n{derive}#[diesel(table_name = {table_name})]\npub struct {struct_name} {{\n    id: i32,\n{attributes}    version: i64\n}}\n\n{id_struct}\n\n",
        struct_comments = comment,
        derive = derive_macros,
        table_name = object.get_table_name(),
        struct_name = object.get_name(),
        attributes = atdef,
    )
}

fn generate_rust_select_clause(object: &Object, depth: usize) -> String {
    let white_space = "    ".repeat(depth);
    let end_space = "    ".repeat(depth - 1);
    let mut select_attribute = String::new();
    let mut separator = format!(
        ".select((\n{white_space}super::schema::{table_name}::id,\n{white_space}",
        table_name = object.get_table_name()
    );
    for at in object.get_attributes() {
        select_attribute += &format!(
            "{sep}super::schema::{table_name}::{attribute_name}",
            sep = separator,
            table_name = object.get_table_name(),
            attribute_name = at.get_name()
        );
        separator = format!(",\n{white_space}");
    }
    if select_attribute.len() > 0 {
        select_attribute += &format!(
            "{sep}super::schema::{table_name}::version\n{end_space}))",
            sep = separator,
            table_name = object.get_table_name()
        );
    }
    select_attribute
}

///Convert a CamelCase string into snake_case
fn get_snake_name(name: &str) -> String {
    let mut snake_name = String::new();
    let mut first = true;
    for c in name.chars() {
        if c.is_uppercase() {
            if first {
                snake_name = format!("{}", c.to_lowercase());
            } else {
                snake_name = format!("{}_{}", snake_name, c.to_lowercase());
            }
        } else {
            snake_name = format!("{}{}", snake_name, c);
        }
        first = false;
    }
    snake_name
}

///Retrieve the underlying rust type used for storage
fn get_base_type(base: &BaseAttributeType) -> String {
    match base {
        BaseAttributeType::Integer => String::from("i64"),
        BaseAttributeType::String => String::from("String"),
        BaseAttributeType::Reference(_) => String::from("i32"),
    }
}

///Retrieve the underlying rust type used for storage
fn get_attribute_type(at_type: &AttributeType) -> String {
    match at_type {
        AttributeType::Mandatory(base) => get_base_type(base),
        AttributeType::Optional(base) => format!("Option<{}>", get_base_type(base)),
    }
}

///Retrieve the expected type in generic rust code, independent
/// of the underlying storage type
fn get_attribute_type_param_type(at_type: &AttributeType) -> String {
    //Retrieve the base attribute type and if it is optional
    let (opt, base) = match at_type {
        AttributeType::Mandatory(base) => (false, base),
        AttributeType::Optional(base) => (true, base),
    };
    //create the rust representation of the base type
    let base_type = match base {
        BaseAttributeType::Integer => String::from("i64"),
        BaseAttributeType::String => String::from("String"),
        BaseAttributeType::Reference(r) => format!("&{}", r),
    };

    if opt {
        format!("Option<{}>", base_type)
    } else {
        base_type
    }
}

/// Retrieve the underlying rust type used for storage
fn get_rust_type(attribute: &Attribute) -> String {
    get_attribute_type(attribute.get_type())
}

/// Retrieve the type that should be used, regardless of the
/// underlying typed used for storage
fn get_rust_param_type(attribute: &Attribute) -> String {
    get_attribute_type_param_type(attribute.get_type())
}

/// Retrieve the type that can be passed as function as a borrowed parameter
fn get_rust_borrowed_type(attribute: &Attribute) -> String {
    match attribute.get_type() {
        AttributeType::Mandatory(base_type) => match base_type {
            BaseAttributeType::String => format!("&str"),
            _ => format!("{}", get_rust_param_type(attribute)),
        },
        AttributeType::Optional(_) => format!("&{}", get_rust_param_type(attribute)),
    }
}

#[cfg(test)]
mod tests {
    use crate::Attribute;

    use super::{get_rust_borrowed_type, get_rust_type};

    #[test]
    fn borrowed_type() {
        //Mandatory attributes
        let at: Attribute = serde_json::from_slice(
            b"{\"name\": \"duration\",\"is\": {\"Mandatory\": \"Integer\"}}",
        )
        .unwrap();
        assert_eq!(String::from("i64"), get_rust_borrowed_type(&at));
        let at: Attribute =
            serde_json::from_slice(b"{\"name\": \"duration\",\"is\": {\"Mandatory\": \"String\"}}")
                .unwrap();
        assert_eq!(String::from("&str"), get_rust_borrowed_type(&at));
        let at: Attribute = serde_json::from_slice(
            b"{\"name\": \"duration\",\"is\": {\"Mandatory\": {\"Reference\":\"Test\"}}}",
        )
        .unwrap();
        assert_eq!(String::from("&Test"), get_rust_borrowed_type(&at));

        //Optional attributes
        let at: Attribute =
            serde_json::from_slice(b"{\"name\": \"duration\",\"is\": {\"Optional\": \"Integer\"}}")
                .unwrap();
        assert_eq!(String::from("&Option<i64>"), get_rust_borrowed_type(&at));
        let at: Attribute =
            serde_json::from_slice(b"{\"name\": \"duration\",\"is\": {\"Optional\": \"String\"}}")
                .unwrap();
        assert_eq!(String::from("&Option<String>"), get_rust_borrowed_type(&at));
        let at: Attribute = serde_json::from_slice(
            b"{\"name\": \"duration\",\"is\": {\"Optional\": {\"Reference\":\"Test\"}}}",
        )
        .unwrap();
        assert_eq!(String::from("&Option<&Test>"), get_rust_borrowed_type(&at));
    }

    #[test]
    fn rust_type() {
        //Mandatory attributes
        let at: Attribute = serde_json::from_slice(
            b"{\"name\": \"duration\",\"is\": {\"Mandatory\": \"Integer\"}}",
        )
        .unwrap();
        assert_eq!(String::from("i64"), get_rust_type(&at));
        let at: Attribute =
            serde_json::from_slice(b"{\"name\": \"duration\",\"is\": {\"Mandatory\": \"String\"}}")
                .unwrap();
        assert_eq!(String::from("String"), get_rust_type(&at));
        let at: Attribute = serde_json::from_slice(
            b"{\"name\": \"duration\",\"is\": {\"Mandatory\": {\"Reference\":\"Test\"}}}",
        )
        .unwrap();
        assert_eq!(String::from("i32"), get_rust_type(&at));

        //Optional attributes
        let at: Attribute =
            serde_json::from_slice(b"{\"name\": \"duration\",\"is\": {\"Optional\": \"Integer\"}}")
                .unwrap();
        assert_eq!(String::from("Option<i64>"), get_rust_type(&at));
        let at: Attribute =
            serde_json::from_slice(b"{\"name\": \"duration\",\"is\": {\"Optional\": \"String\"}}")
                .unwrap();
        assert_eq!(String::from("Option<String>"), get_rust_type(&at));
        let at: Attribute = serde_json::from_slice(
            b"{\"name\": \"duration\",\"is\": {\"Optional\": {\"Reference\":\"Test\"}}}",
        )
        .unwrap();
        assert_eq!(String::from("Option<i32>"), get_rust_type(&at));
    }
}
