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
use crate::{Object, ObjectDB, Error};

use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

pub fn write_install(object_db: &ObjectDB, destination: &str, doc_name: &str) -> Result<(), Error> {
    let pbuf = PathBuf::from(destination);
    if !pbuf.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: String::from(destination),
        });
    }

    let content = generate_sqlite_install(object_db);

    let (mut sql_file, filename) = get_sqlite_file(&pbuf, doc_name)?;

    match sql_file.write(content.as_bytes()) {
        Err(_e) => {
            return Err(Error::UnableToWriteToFile {
                file: filename,
                content,
            })
        }
        Ok(size_written) => {
            if size_written != content.as_bytes().len() {
                return Err(Error::UnableToWriteToFile {
                    file: filename,
                    content,
                });
            }
        }
    };
    Ok(())
}

fn get_sqlite_file(dir: &PathBuf, doc_name: &str) -> Result<(File, String), Error> {
    if !dir.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: match dir.to_str() {
                Some(p) => String::from(p),
                None => String::from("UNKNOWN"),
            },
        });
    }

    let mut destination = PathBuf::from(dir);

    destination.push(doc_name);
    destination.set_extension("sql");

    let filename = match destination.as_path().to_str() {
        Some(pth) => String::from(pth),
        None => format!("{}.sql", doc_name),
    };

    match File::create(destination.as_path()) {
        Err(_e) => {
            return Err(Error::UnableToCreateFile {
                file: filename,
            })
        }
        Ok(f) => Ok((f, filename)),
    }
}

//Generate the sqlite directives to create the necessary table for the datamodel
pub fn generate_sqlite_install(db: &ObjectDB) -> String {
    let mut tables = String::new();
    let mut indexes = String::new();
    for obj in db.get_objects() {
        tables = format!("{tables}\n{obj_table}", obj_table = sqlite_table(obj));
        indexes = format!(
            "{indexes}\n{obj_indexes}",
            obj_indexes = sqlite_indexes(obj)
        );
    }
    return format!("{tables}\n{indexes}");
}

fn sqlite_table(obj: &Object) -> String {
    let table_name = obj.get_table_name();
    let mut columns = String::new();
    let mut foreign_keys = String::new();
    let mut uniques = String::new();
    for attribute in obj.get_attributes() {
        let column_name = attribute.get_name();
        let nullable = match attribute.get_type() {
            crate::AttributeType::Mandatory(_) => String::from(" NOT NULL"),
            crate::AttributeType::Optional(_) => String::new(),
        };
        let sql_type = match attribute.get_type().get_base_type() {
            crate::BaseAttributeType::Integer => String::from("INTEGER"),
            crate::BaseAttributeType::String => String::from("VARCHAR"),
            crate::BaseAttributeType::Reference(refered) => {
                foreign_keys = format!(
                    "{previous},\n    FOREIGN KEY({column_name}) REFERENCES {refered}(id)",
                    previous = foreign_keys
                );
                String::from("INTEGER")
            }
        };
        if attribute.is_indexable() {
            uniques = format!("{uniques},\n    UNIQUE({column_name})");
        }
        columns = format!("{columns},\n    {column_name} {sql_type}{nullable}")
    }
    format!(
        "CREATE TABLE {table_name} (\n    id INTEGER PRIMARY KEY NOT NULL{columns},    version INTEGER NOT NULL{foreign_keys}{uniques}\n);\n",
        
    )
}

fn sqlite_indexes(obj: &Object) -> String {
    let table_name = obj.get_table_name();
    let mut indexes = String::new();
    for attribute in obj.get_attributes() {
        if attribute.is_indexable() {
            let column_name = attribute.get_name();
            indexes = format!("{indexes}\nCREATE INDEX idx_{table_name}_{column_name} ON {table_name}({column_name});");
        }
    }
    indexes
}
