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
use crate::objectdb::ObjectDB;
use crate::{Attribute, AttributeType, BaseAttributeType};

pub fn generate_rust_impl(object: &Object, db: &ObjectDB) -> String {
    format!(
        "impl {} {{\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n}}",
        object.get_name(),
        newfn(object),
        createfn(object),
        mass_create(object),
        deletefn(object),
        loadfn(object),
        nbdefinedfn(object),
        loadmultiplefn(object),
        load_all(object),
        gettersfn(object),
        settersfn(object),
        get_relations(object, db),
    )
}

fn loadmultiplefn(object: &Object) -> String {
    format!(
        "
    ///Load up to a given number of instances, starting with a given id
    pub fn load_multiple(
        from: i32,
        to: i32,
        connection: &mut SqliteConnection,
    ) -> Result<Vec<{object_name}>, bdmg::Error> {{
        let result = {table_name}::dsl::{table_name}
            {select_clause}
            .order({table_name}::id.asc())
            .limit((to - from).into())
            .offset(from.into())
            .load::<{object_name}>(connection)?;
        Ok(result)
    }}",
        object_name = object.get_name(),
        table_name = object.get_table_name(),
        select_clause = super::generate_rust_select_clause(object, 3),
    )
}

fn load_all(object: &Object) -> String {
    format!(
        "
    /// Load all instances present in the database
    pub fn load_all(connection: &mut SqliteConnection) -> Result<Vec<{object_name}>, bdmg::Error> {{
        let result = {table_name}::dsl::{table_name}
            {select_clause}
            .order({table_name}::id.asc())
            .load::<{object_name}>(connection)?;
        Ok(result)
    }}",
        object_name = object.get_name(),
        table_name = object.get_table_name(),
        select_clause = super::generate_rust_select_clause(object, 3),
    )
}

fn gettersfn(object: &Object) -> String {
    let mut getters = format!(
        "    ///Retrieve the id of this instance
    pub fn id(&self) -> Id{ObjectName} {{
        Id{ObjectName} {{
            id: self.id
        }}
    }}\n",
        ObjectName = object.get_name()
    );
    for at in object.get_attributes() {
        getters = getters + &attribute_getter(at);
    }
    getters
}

fn attribute_getter(attribute: &Attribute) -> String {
    let comment = format!(
        "/// Retrieve the value of the {attribute_name} attribute.{desc}\n    ",
        attribute_name = attribute.get_name(),
        desc = match attribute.get_comment() {
            Some(comm) => format!("\n    /// {comm}"),
            None => String::new(),
        }
    );
    let attribute_name = attribute.get_name();
    match attribute.get_reference() {
        Some(ref_object) => match attribute.get_type() {
            AttributeType::Mandatory(_) => {
                format!("\n
    {comment}pub fn get_{attribute_name}(&self, connection: &mut SqliteConnection) -> Result<{ref_object},bdmg::Error> {{ {ref_object}::load(connection, self.{attribute_name}) }}
    {comment}pub fn get_{attribute_name}_id(&self) -> Id{ref_object} {{ Id{ref_object} {{ id: self.{attribute_name} }} }}")
            }
            AttributeType::Optional(_) => {
                let body = format!("
        match self.{attribute_name} {{
            None => None,
            Some(id) => match {ref_object}::load(connection, id) {{ Err(_e) => None, Ok(v) => Some(v) }}
        }}");
                format!("\n
    {comment}pub fn get_{attribute_name}(&self, connection: &mut SqliteConnection) -> Option<{ref_object}> {{ {body} }}
    {comment}pub fn get_{attribute_name}_id(&self) -> Option<Id{ref_object}> {{
        match self.{attribute_name} {{
            Some(v) => Some(Id{ref_object} {{ id: v }}),
            None => None
        }}
    }}")
            }
        },
        None => {
            if attribute.get_type().get_base_type() == &BaseAttributeType::Integer {
                format!(
                    "\n    {comment}pub fn get_{}(&self) -> {} {{ self.{} }}",
                    attribute.get_name(),
                    super::get_rust_type(attribute),
                    attribute.get_name()
                )
            } else {
                format!(
                    "\n    {comment}pub fn get_{}(&self) -> &{} {{ &self.{} }}",
                    attribute.get_name(),
                    super::get_rust_type(attribute),
                    attribute.get_name()
                )
            }
        }
    }
}

fn settersfn(object: &Object) -> String {
    let mut setters = String::new();
    for at in object.get_attributes() {
        setters = setters + &atribute_setter(object, at, object.get_table_name());
    }
    setters
}

/// Generate the rust setter for the attribute
fn atribute_setter<'a>(object: &Object, attribute: &Attribute, table_name: &'a String) -> String {
    let comment = format!(
        "/// Set the value of the {attribute_name} attribute.{desc}\n    ",
        attribute_name = attribute.get_name(),
        desc = match attribute.get_comment() {
            Some(comm) => format!("\n    /// {comm}"),
            None => String::new(),
        }
    );
    if !attribute.is_mutable() {
        return String::new();
    }
    let declaration = format!("\n    {comment}pub fn set_{name}(&mut self, attr_value : {rust_type}, connection: &mut SqliteConnection) -> Result<(), bdmg::Error> {{",
        name = attribute.get_name(),
        rust_type = super::get_rust_param_type(attribute),
        );

    let check_if_needed = match attribute.get_reference() {
        None => format!(
            "\n        if attr_value == self.{name} {{ return Ok(());}}\n",
            name = attribute.get_name()
        ),
        Some(_) => match attribute.get_type() {
            AttributeType::Mandatory(_) => {
                format!(
                    "\n        if attr_value.get_id() == self.{name} {{ return Ok(());}}\n",
                    name = attribute.get_name()
                )
            }
            AttributeType::Optional(_) => {
                format!("\n        if (attr_value.is_none() && self.{name}.is_none()) || (attr_value.is_some() && Some(attr_value.unwrap().get_id()) == self.{name}) {{ return Ok(()); }} ",
                        name = attribute.get_name())
            }
        },
    };

    let validator = match object.get_validator() {
        None => String::new(),
        Some(function) => format!("\nif ! {function}(connection, &t) {{ return Err(bdmg::Error::UnableToCreateRecord(String::from(\"validation failed\"))); }}\n")
    };

    let update_query = format!(
        "
        let result = diesel::update(
            {table}::dsl::{table}.filter(
                {table}::id
                    .eq(self.id)
                    .and({table}::version.eq(self.version)),
            ),
        ).set((
            {table}::{attr_name}.eq({value}),
            {table}::version.eq(self.version + 1),
        ))
            .execute(connection);",
        table = table_name,
        attr_name = attribute.get_name(),
        value = {
            match attribute.get_reference() {
                Some(_) => match attribute.get_type() {
                    AttributeType::Mandatory(_) => "attr_value.get_id()",
                    AttributeType::Optional(_) => {
                        "match attr_value { Some(val) => Some(val.get_id()), None => None } "
                    }
                },
                None => "&attr_value",
            }
        },
    );

    let result_treatment = format!(
        "
        match result {{
            Err(e) => Err(bdmg::Error::InternalError(e)),
            Ok(v) => {{
                if v == 1 {{
                    self.version += 1;
                    self.{attr_name} = {attr_value};
                    Ok(())
                }} else {{
                    Err(bdmg::Error::ElementNotFound)
                }}
            }}
        }}
    }}\n",
        attr_name = attribute.get_name(),
        attr_value = {
            match attribute.get_reference() {
                Some(_) => match attribute.get_type() {
                    AttributeType::Mandatory(_) => "attr_value.get_id()",
                    AttributeType::Optional(_) => {
                        "match attr_value { Some(val) => Some(val.get_id()), None => None } "
                    }
                },
                None => "attr_value",
            }
        }
    );

    declaration + &check_if_needed + &validator + &update_query + &result_treatment
}

fn nbdefinedfn(object: &Object) -> String {
    format!("
    /// Retrieve the number of instances present on database
    pub fn get_nb_defined(connection: &mut SqliteConnection) -> i64 {{
        match {table_name}::dsl::{table_name}.select(diesel::dsl::count({table_name}::id)).limit(1).get_result::<i64>(connection) {{
            Ok(v)   => v,
            Err(_e) => 0
        }}
    }}",
        table_name = object.get_table_name())
}

fn loadfn(object: &Object) -> String {
    let mut loaders = String::new();

    //Create the loader based on the content
    match object.is_object_immutable_relation() {
        Some((first, second)) => {
            let first_type = first.get_reference().unwrap();
            let first_name = first.get_name();
            let first_arg_name = first_type.to_ascii_lowercase();
            let second_type = second.get_reference().unwrap();
            let second_name = second.get_name();
            let second_arg_name = second_type.to_ascii_lowercase();
            let table_name = object.get_table_name();

            loaders = loaders + &format!("
        /// Load an instance based on the content of this object
    pub fn load_from_content(connection: &mut SqliteConnection, a_{first_arg_name}: Id{first_type}, a_{second_arg_name}: Id{second_type}) -> Result<Vec<{object_name}>, bdmg::Error> {{
        return Ok({table_name}::dsl::{table_name}
            .filter(super::schema::{table_name}::{first_name}.eq(a_{first_arg_name}.id))
            .filter(super::schema::{table_name}::{second_name}.eq(a_{second_arg_name}.id))
            .load::<{object_name}>(connection)?);
    }}",object_name = object.get_name(),)
        }
        None => {}
    }

    //Create the loaders based on attributes that are indexable references
    for at in object.get_attributes() {
        if at.is_indexable() {
            let value = match at.get_reference() {
                Some(_referenced_name) => String::from("attribute.get_id()"),
                None => String::from("&attribute"),
            };
            loaders = loaders
                        + &format!(
                            "
    /// Load an instance based on the attribute {attribute_name}
    pub fn load_from_{attribute_name}(connection: &mut SqliteConnection, attribute: {borrowed_type}) -> Result<{object_name}, bdmg::Error> {{
        let mut result = {table_name}::dsl::{table_name}
            .filter({table_name}::{attribute_name}.eq({attribute_value}))
            .limit(1)
            .load::<{object_name}>(connection)?;
        match result.len() {{
            1 => Ok(result.pop().unwrap()),
            _ => Err(bdmg::Error::ElementNotFound)
        }}
    }}",
                        attribute_name = at.get_name(),
                        attribute_value = value,
                        borrowed_type = super::get_rust_borrowed_type(at),
                        object_name = object.get_name(),
                        table_name = object.get_table_name(),
                        );
        }
    }
    format!(
        "
    /// Load an instance based on its identifier
    pub fn load(connection: &mut SqliteConnection, identifier: i32) -> Result<{object_name}, bdmg::Error> {{
        let result = {table_name}::dsl::{table_name}
            {select_clause}
            .filter({table_name}::id.eq(identifier))
            .limit(1)
            .load::<{object_name}>(connection);
        match result {{
            Err(e) => Err(bdmg::Error::InternalError(e)),
            Ok(mut v) => {{
                if v.len() != 1 {{
                    Err(bdmg::Error::ElementNotFound)
                }} else {{
                    Ok(v.pop().unwrap())
                }}
            }}
        }}
    }}

    /// Load an instance based on its typed identifier
    pub fn load_from_id(connection: &mut SqliteConnection, identifier: Id{object_name}) -> Result<{object_name}, bdmg::Error> {{
        Self::load(connection, identifier.id)
    }}
{load_from_attribute}",
        object_name = object.get_name(),
        table_name = object.get_table_name(),
        select_clause = super::generate_rust_select_clause(object, 3),
        load_from_attribute = loaders,
    )
}

fn deletefn(object: &Object) -> String {
    format!(
        "
    /// Delete the instance on the database and consume the rust instance to make sure it can't be used aferwards
    pub fn delete<'a>(self, connection: &'a mut SqliteConnection) -> Result<(), bdmg::Error> {{
        diesel::delete({table_name}::dsl::{table_name}.filter({table_name}::id.eq(self.id))).execute(connection)?;
        Ok(())
    }}",
    table_name = object.get_table_name()
    )
}

fn newfn(object: &Object) -> String {
    let function_params = {
        //declaration of function parameters
        let mut params = String::new();
        for at in object.get_attributes() {
            match at.get_type() {
                AttributeType::Mandatory(_) => match at.get_reference() {
                    Some(r) => {
                        params = params + &format!("\n        a_{}: &{},", at.get_name(), r);
                    }
                    None => {
                        params = params
                            + &format!(
                                "\n        a_{}: {},",
                                at.get_name(),
                                super::get_rust_param_type(at)
                            );
                    }
                },
                AttributeType::Optional(_) => match at.get_reference() {
                    Some(r) => {
                        params =
                            params + &format!("\n        a_{}: Option<&{}>,", at.get_name(), r);
                    }
                    None => {
                        params = params
                            + &format!(
                                "\n        a_{}: {},",
                                at.get_name(),
                                super::get_rust_param_type(at)
                            );
                    }
                },
            }
        }
        params
    };
    let constructor_param = {
        let mut params = String::new();
        for at in object.get_attributes() {
            match at.get_type() {
                AttributeType::Mandatory(_) => match at.get_reference() {
                    Some(_r) => {
                        params = params
                            + &format!("a_{attribute_name}.id(),", attribute_name = at.get_name());
                    }
                    None => {
                        params = params
                            + &format!("a_{attribute_name},", attribute_name = at.get_name());
                    }
                },
                AttributeType::Optional(_) => match at.get_reference() {
                    Some(_r) => {
                        params = params
                        + &format!(
                            "if a_{attribute_name}.is_some() {{ Some(a_{attribute_name}.unwrap().id())}} else {{ None }},",
                            attribute_name = at.get_name()
                        );
                    }
                    None => {
                        params = params
                            + &format!("a_{attribute_name},", attribute_name = at.get_name());
                    }
                },
            }
        }
        params
    };

    format!(
        "
    /// Create a new instance of {object_name}
    pub fn new(
        connection: &mut SqliteConnection,{function_parameters}
    ) -> Result<{object_name}, bdmg::Error> {{
        Self::create(connection, {constructor})
    }}",
        function_parameters = function_params,
        object_name = object.get_name(),
        constructor = constructor_param
    )
}

fn createfn(object: &Object) -> String {
    let function_params = {
        //declaration of function parameters
        let mut params = String::new();
        for at in object.get_attributes() {
            match at.get_type() {
                AttributeType::Mandatory(_) => match at.get_reference() {
                    Some(r) => {
                        params = params + &format!("\n        a_{}: Id{},", at.get_name(), r);
                    }
                    None => {
                        params = params
                            + &format!(
                                "\n        a_{}: {},",
                                at.get_name(),
                                super::get_rust_param_type(at)
                            );
                    }
                },
                AttributeType::Optional(_) => match at.get_reference() {
                    Some(r) => {
                        params =
                            params + &format!("\n        a_{}: Option<Id{}>,", at.get_name(), r);
                    }
                    None => {
                        params = params
                            + &format!(
                                "\n        a_{}: {},",
                                at.get_name(),
                                super::get_rust_param_type(at)
                            );
                    }
                },
            }
        }
        params
    };
    let constructor_param = {
        let mut params = String::new();
        for at in object.get_attributes() {
            match at.get_type() {
                AttributeType::Mandatory(_) => match at.get_reference() {
                    Some(_r) => {
                        params = params
                            + &format!(
                                "\n            {attribute_name}: a_{attribute_name}.id,",
                                attribute_name = at.get_name()
                            );
                    }
                    None => {
                        params = params
                            + &format!(
                                "\n            {attribute_name}: a_{attribute_name},",
                                attribute_name = at.get_name()
                            );
                    }
                },
                AttributeType::Optional(_) => match at.get_reference() {
                    Some(_r) => {
                        params = params
                        + &format!(
                            "\n            {attribute_name}: if a_{attribute_name}.is_some() {{ Some(a_{attribute_name}.unwrap().id)}} else {{ None }},",
                            attribute_name = at.get_name()
                        );
                    }
                    None => {
                        params = params
                            + &format!(
                                "\n            {attribute_name}: a_{attribute_name},",
                                attribute_name = at.get_name()
                            );
                    }
                },
            }
        }
        params
    };

    let validator = match object.get_validator() {
        None => String::new(),
        Some(function) => format!("if ! {function}(connection, &t) {{ return Err(bdmg::Error::UnableToCreateRecord(format!(\"validation failed for {{:?}}\", t))); }}")
    };

    let insertable_creation = match object.get_validator() {
        None => format!(
            "let tmp = Insertable{object_name} {{{constructor_param}
            version: 0,
        }};",
            object_name = object.get_name()
        ),
        Some(_function) => format!(
            "let t = {object_name} {{
            id: 0,{constructor_param}
            version: 0,
        }};
        {validator}
        let tmp = Insertable{object_name}::from(t);",
            object_name = object.get_name()
        ),
    };

    format!(
        "
    /// Create a new instance of {object_name} based on the ids of the references (if any)
    pub fn create(
        connection: &mut SqliteConnection,{function_params}
    ) -> Result<{object_name}, bdmg::Error> {{
        {insertable_creation}

        let result = diesel::insert_into({table_name}::table)
            .values(&tmp)
            .returning({table_name}::id)
            .get_result::<i32>(connection);

        match result {{
            Ok(id) => {{
                Ok({object_name}::from((id, tmp)))
            }}
            Err(e) => {{
                Err(bdmg::Error::UnableToCreateRecord(format!(
                    \"Error while creating instance {{:?}}: {{e}}\",
                    {object_name}::from((-1, tmp))
                )))
            }}
        }}
    }}",
        object_name = object.get_name(),
        table_name = object.get_table_name(),
    )
}

fn mass_create(object: &Object) -> String {
    let pair_type = {
        //declaration of function parameters
        let mut params = String::new();
        for at in object.get_attributes() {
            match at.get_type() {
                AttributeType::Mandatory(_) => match at.get_reference() {
                    Some(r) => {
                        params = params + &format!("Id{r},");
                    }
                    None => {
                        params = params + &format!("{},", super::get_rust_param_type(at));
                    }
                },
                AttributeType::Optional(_) => match at.get_reference() {
                    Some(r) => {
                        params = params + &format!("Option<Id{}>,", r);
                    }
                    None => {
                        params = params + &format!("{},", super::get_rust_param_type(at));
                    }
                },
            }
        }
        params
    };
    let constructor_param = {
        let mut params = String::new();
        for (index, at) in object.get_attributes().enumerate() {
            let attribute_name = at.get_name();
            match at.get_type() {
                AttributeType::Mandatory(_) => match at.get_reference() {
                    Some(_r) => {
                        params = params
                            + &format!("\n                {attribute_name}: element.{index}.id,");
                    }
                    None => {
                        params = params
                            + &format!("\n                {attribute_name}: element.{index},");
                    }
                },
                AttributeType::Optional(_) => match at.get_reference() {
                    Some(_r) => {
                        params = params
                        + &format!(
                            "\n                {attribute_name}: if element.{index}.is_some() {{ Some(element.{index}.unwrap().id)}} else {{ None }},"
                        );
                    }
                    None => {
                        params = params
                            + &format!("\n                {attribute_name}: element.{index},");
                    }
                },
            }
        }
        params
    };

    let validator = match object.get_validator() {
        None => String::new(),
        Some(function) => format!("if ! {function}(connection, &t) {{ return Err(bdmg::Error::UnableToCreateRecord(String::from(\"validation failed\"))); }}")
    };

    let insertable_creation = match object.get_validator() {
        None => format!(
            "let tmp = Insertable{object_name} {{{constructor_param}
                version: 0,
            }};",
            object_name = object.get_name()
        ),
        Some(_function) => format!(
            "let t = {object_name} {{
            id: 0,{constructor_param}
                version: 0,
            }};
            {validator}
            let tmp = Insertable{object_name}::from(t);",
            object_name = object.get_name()
        ),
    };

    format!(
        "
    /// Create multiple new instances of {object_name} based on the ids of the references (if any)
    pub fn mass_create(
        connection: &mut SqliteConnection,
        values: Vec<({pair_type})>,
    ) -> Result<(), bdmg::Error> {{
        if values.is_empty() {{
            return Ok(());
        }}
        let mut new_values = Vec::with_capacity(values.len());
        for element in values {{
            {insertable_creation}
            new_values.push(tmp);
        }}
        let result = diesel::insert_into({table_name}::table)
            .values(&new_values)
            .execute(connection);
        match result {{
            Ok(_) => Ok(()),
            Err(e) => Err(bdmg::Error::UnableToCreateRecord(format!(\"Unable to mass create instnaces of {object_name}: {{e}}\"))),
        }}
    }}",
        object_name = object.get_name(),
        table_name = object.get_table_name(),
    )
}

fn get_relations(object: &Object, db: &ObjectDB) -> String {
    let mut code = String::new();
    for referencing_name in object.get_referencing_objects() {
        let referencing = match db.get_object(referencing_name) {
            Some(obj) => obj,
            None => continue,
        };
        let attribute_referencing_object =
            match referencing.get_relation_attribute(object.get_name()) {
                Some(at) => at.get_name(),
                None => continue,
            };
        let referencing_snake_name = super::get_snake_name(referencing_name);
        let function_name = format!("get_{}s", referencing_snake_name);
        let select_clause = super::generate_rust_select_clause(referencing, 4);
        code = format!(
            "{existing}    ///Retrieve all {referencing_name} referencing this object
    pub fn {function_name}(
        self: &{object_name},
        connection: &mut SqliteConnection,
    ) -> Result<Vec<super::{referencing_name}>, bdmg::Error> {{
        Ok(super::schema::{referencing_table}::dsl::{referencing_table}
            {select}
            .filter(super::schema::{referencing_table}::{attribute_name}.eq(self.id))
            .load::<super::{referencing_name}>(connection)?)
    }}\n",
            existing = code,
            function_name = function_name,
            referencing_name = referencing_name,
            object_name = object.get_name(),
            referencing_table = referencing.get_table_name(),
            select = select_clause,
            attribute_name = attribute_referencing_object
        );

        match referencing.is_object_relation() {
            None => {}
            Some((a, b)) => {
                let dest = if a == object.get_name() {
                    db.get_object(b)
                } else {
                    db.get_object(a)
                };
                match dest {
                    None => {}
                    Some(dest_object) => {
                        let destination_object_name = dest_object.get_name();
                        let function_name = format!(
                            "get_{}s_from_{}s",
                            super::get_snake_name(destination_object_name),
                            referencing_snake_name
                        );
                        let mut selected_attributes = String::new();
                        for at in dest_object.get_attributes() {
                            selected_attributes = format!("{selected_attributes}                    super::schema::{destination_table}::{attribute_name},\n", destination_table = dest_object.get_table_name(), attribute_name = at.get_name());
                        }
                        code = format!("{existing}\n    ///Retrieve all {destination_name} that related to this object through {referencing_name}
    pub fn {function_name}(
        self: &{object_name},
        connection: &mut SqliteConnection,
    ) -> Result<Vec<super::{destination_name}>, bdmg::Error> {{
        Ok(super::schema::{destination_table}::table
            .select((
                super::schema::{destination_table}::id,
{selected_attributes}                    super::schema::{destination_table}::version,
            ))
            .inner_join(super::schema::{rel_table}::table)
            .filter(super::schema::{rel_table}::{att_name}.eq(self.id))
            .load::<super::{destination_name}>(connection)?)
    }}\n", 
                            existing = code,
                            function_name = function_name,
                            object_name = object.get_name(),
                            referencing_name = referencing_name,
                            destination_name = destination_object_name,
                            destination_table = dest_object.get_table_name(),
                            rel_table = referencing.get_table_name(),
                            att_name = referencing.get_relation_attribute(object.get_name()).unwrap().get_name()
                        );
                    }
                }
            }
        }
    }
    code
}
