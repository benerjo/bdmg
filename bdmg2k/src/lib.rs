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

#[macro_use]
extern crate serde_derive;

extern crate serde_json;

mod attributes;
mod object;
mod objectdb;

pub mod doc_generator;
pub mod rust_generator;
pub mod sqlite_generator;

pub use attributes::*;
pub use object::Object;
pub use objectdb::ObjectDB;

#[derive(Debug)]
pub enum Error {
    UnableToCreateOutputDirectory {
        destination: String,
        error: std::io::Error,
    },
    DestinationIsNotDirectory {
        destination: String,
    },
    NoDestinationDirectorySpecified,
    UnableToCreateFile {
        file: String,
    },
    UnableToWriteToFile {
        file: String,
        content: String,
    },
    UnableToWriteCodeForObject {
        object_name: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnableToCreateOutputDirectory { destination, error } => write!(
                f,
                "Unable to create the directory '{destination}': Error: {error}"
            ),
            Error::DestinationIsNotDirectory { destination } => {
                write!(f, "The destination '{destination}' is not a directory.")
            }
            Error::UnableToCreateFile { file } => {
                write!(f, "Unable to create the file '{file}'.")
            }
            Error::UnableToWriteToFile { file, content } => write!(
                f,
                "Unable to write to the file '{file}': >>>{}",
                content.replace("\n", "\n>>>")
            ),
            Error::UnableToWriteCodeForObject { object_name } => write!(
                f,
                "Unable to create the code for the object '{object_name}'."
            ),
            Error::NoDestinationDirectorySpecified => write!(
                f,
                "No destination directory has been set for the code to be generated in."
            ),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub enum ValidationError {
    UnknownReferencedType {
        referenced_type: String,
        object: String,
        attribute: String,
    },
    ObjectReferecendedMultipleTimes {
        referenced_type: String,
        object_referencing: String,
        attributes: Vec<String>,
    },
    ObjectIsUsingReservedName {
        object_name: String,
    },
    AttributeIsUsingReservedName {
        object_name: String,
        attribute_name: String,
    },
    TableNameIsInvalid {
        object_name: String,
        table_name: String,
    },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::UnknownReferencedType {
                referenced_type,
                object,
                attribute,
            } => write!(
                f,
                "Unknown referenced type '{referenced_type}' in '{object}.{attribute}'"
            ),
            ValidationError::ObjectReferecendedMultipleTimes {
                referenced_type,
                object_referencing,
                attributes,
            } => {
                write!(f,
                                        "Object type '{referenced_type}' is being referenced multiple times in '{object_referencing}'.["
                                    )?;
                for a in attributes {
                    write!(f, "'{a}' ")?;
                }
                write!(f, "]")
            }
            ValidationError::ObjectIsUsingReservedName { object_name } => {
                write!(f, "Object '{object_name}' is using a reserved name")
            }
            ValidationError::AttributeIsUsingReservedName {
                object_name,
                attribute_name,
            } => write!(
                f,
                "Attribute '{attribute_name}' in '{object_name}' is using a reserved name"
            ),
            ValidationError::TableNameIsInvalid {
                object_name,
                table_name,
            } => write!(
                f,
                "The table name '{table_name}' related to the object '{object_name}' is invalid"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

///Retrieve or create a given file
/// Return a pair containing the std::fs::File and the filename
fn get_file(
    dir: &std::path::PathBuf,
    filename: &str,
    extension: &str,
) -> Result<(std::fs::File, String), Error> {
    if !dir.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: match dir.to_str() {
                Some(p) => String::from(p),
                None => String::from("UNKNOWN"),
            },
        });
    }

    let mut destination = std::path::PathBuf::from(dir);

    destination.push(filename);
    destination.set_extension(extension);

    let filename = match destination.as_path().to_str() {
        Some(pth) => String::from(pth),
        None => format!("{filename}.{extension}"),
    };

    match std::fs::File::create(destination.as_path()) {
        Err(_e) => return Err(Error::UnableToCreateFile { file: filename }),
        Ok(f) => Ok((f, filename)),
    }
}
