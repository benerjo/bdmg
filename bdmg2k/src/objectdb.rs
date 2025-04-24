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

use crate::object::Object;
use crate::rust_generator;

use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::path::Path;

///The type of output the generator should create
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
pub enum RustOutputType {
    Module,
    Library,
}

///The container of all objects that needs to be defined
#[derive(Serialize, Deserialize, Debug)]
pub struct ObjectDB {
    ///The directory in which the code will be generated
    rust_destination: String,
    ///The type of output that must be generated
    rust_output: Option<RustOutputType>,
    ///The list of objects
    objects: Vec<Object>,
    ///The mapping between an object's name and the index
    /// of its structured representation in the objects Vec
    #[serde(skip)]
    objects_position: BTreeMap<String, usize>,
}

impl ObjectDB {
    ///Load the object datbase defined as json in a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<ObjectDB, Box<dyn Error>> {
        use serde_json::from_reader;
        // Open the file in read-only mode.
        let file = File::open(path)?;

        // Read the JSON contents of the file as an instance of object store.
        let mut db: ObjectDB = from_reader(file)?;

        // create the mapping representing the relations
        let mut relations = BTreeMap::<String, Vec<String>>::new();
        let mut index = 0;
        let mut objects_position = BTreeMap::<String, usize>::new();
        for obj in &db.objects {
            objects_position.insert(obj.get_name().clone(), index);
            index += 1;
            let mut refered_objects = vec![];
            for at in obj.get_attributes() {
                match at.get_reference() {
                    Some(r) => refered_objects.push(r.clone()),
                    None => {}
                }
            }
            if refered_objects.len() > 0 {
                relations.insert(obj.get_name().clone(), refered_objects);
            }
        }

        db.objects_position = objects_position;

        // fill in the relations
        for (referencing, refereds) in &relations {
            for refered in refereds {
                let refered_object = match db.get_object_mut(refered) {
                    Some(o) => o,
                    None => continue,
                };
                refered_object.add_referencing_object(referencing.clone());
            }
        }

        // Return the object store.
        Ok(db)
    }

    ///Retrieve a mutable reference to an object from its name, if it exists
    fn get_object_mut(&mut self, name: &str) -> Option<&mut Object> {
        let index = *self.objects_position.get(name)?;
        self.objects.get_mut(index)
    }

    ///Retrieve the object description from its name
    pub fn get_object(&self, name: &str) -> Option<&Object> {
        let index = *self.objects_position.get(name)?;
        self.objects.get(index)
    }

    ///Make sure that all referenced objects are existing in this object store
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::HashMap;
        let mut objects_map = HashMap::new();
        for obj in &self.objects {
            objects_map.insert(obj.get_name(), obj);
        }
        for obj in &self.objects {
            match obj.validate(&objects_map) {
                Ok(_) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    ///Retrieve the iterator to the different objects defined in this object database
    pub fn get_objects(&self) -> std::slice::Iter<'_, Object> {
        self.objects.iter()
    }

    ///Generate the rust code for all objects
    pub fn generate(&self) -> Result<(), String> {
        match rust_generator::generate_code(
            &self,
            &self.rust_destination,
            match &self.rust_output {
                Some(t) => *t,
                None => RustOutputType::Module,
            },
        ) {
            Ok(()) => Ok(()),
            Err(e) => Err(format!("{:?}", e)),
        }
    }
}
