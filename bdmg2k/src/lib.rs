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
    UnableToCreateFile {
        file: String,
    },
    UnableToWriteToFile {
        file: std::fs::File,
        content: String,
    },
    UnableToWriteCodeForObject {
        object_name: String,
    },
}
