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
use crate::{AttributeType, BaseAttributeType, ObjectDB};

/// Generate the different rust traits impl that are needed for the object:
/// SqlRepresentation, Object, ObjectIntrospection and ObjectFactory
pub fn generate_traits_impl(object: &Object, db: &ObjectDB) -> String {
    let object_intro_struct_impl = format!(
            "/// An empty structure whose sole purpose is to provide an implementation of the ObjectIntrospection trait for {object_name}\nstruct {object_name}ObjectIntrospection {{ }}

impl ObjectIntrospection for {object_name}ObjectIntrospection {{
    {impl_code}
}}",
            object_name = object.get_name(),
            impl_code = generate_object_introspection_traits_impl(object, db)
        );
    format!(
        "{}\n\n{}\n\n{}\n\n{}\n{}\n\n",
        generate_sqlrepresentation_traits_impl(object),
        generate_object_traits_impl(object),
        object_intro_struct_impl,
        generate_object_factory_struct(object),
        generate_object_factory_traits_impl(object)
    )
}

// ObjectIntrospection

fn generate_object_introspection_traits_impl(object: &Object, db: &ObjectDB) -> String {
    let mut attr_name_list = format!("        let {}attrs = Vec::new();\n", if object.has_public_attributes(){"mut "} else {""});
    for at in object.get_attributes() {
        if !at.is_secret() {
            attr_name_list = format!(
                "{}        attrs.push(String::from(\"{}\"));\n",
                attr_name_list,
                at.get_name()
            );
        }
    }
    let mut attr_list = format!("        let {}attrs = Vec::new();
    ", if object.has_public_attributes(){"mut "} else {""});
    for at in object.get_attributes() {
        if !at.is_secret() {
            let attribute_type = match at.get_type().get_base_type() {
                BaseAttributeType::Integer => String::from("Integer"),
                BaseAttributeType::String => String::from("String"),
                BaseAttributeType::Reference(r) => {
                    format!("Reference(String::from(\"{}\"))", r)
                }
            };
            attr_list = format!(
                "{list}    attrs.push(bdmg::Attribute::new(
            String::from(\"{name}\"),
            bdmg::AttributeType::{at_type},
            {optional},
            {mutable}));\n    ",
                list = attr_list,
                name = at.get_name(),
                at_type = attribute_type,
                optional = at.is_optional(),
                mutable = at.is_mutable()
            )
        }
    }
    let mut atdef = String::new();
    for at in object.get_attributes() {
        atdef = atdef + "            " + at.get_name() + ": None,\n";
    }
    let object_name = object.get_name();
    let get_object_code = format!(
        "match {object_name}::load(connection, id) {{
            Ok(obj) => {{
                if version.is_none() || obj.get_version() == version.unwrap() {{
                    Ok(Box::new(obj))
                }} else {{
                    Err(bdmg::Error::InvalidVersion)
                }}
            }},
            Err(e) => Err(e)
        }}"
    );
    format!(
        "fn get_attribute_names(&self) -> Vec<String> {{
{attr_name_list}
        return attrs;
    }}
    
    fn get_attributes(&self) -> Vec<bdmg::Attribute> {{
{attr_list}
        return attrs;
    }}
    
    fn get_object_name(&self) -> String {{
        String::from(\"{object_name}\")
    }}
    
    fn create_factory<'a>(
        &self
    ) -> Box<(dyn ObjectFactory + 'static)> {{
        Box::new({object_name}ObjectFactory {{
    {atdef}        }})
    }}
    
    fn get_object(
        &self,
        connection: &mut diesel::sqlite::SqliteConnection,
        id: i32,
        version: Option<i64>
    ) -> Result<Box<(dyn Object + 'static)>, bdmg::Error> {{
        {get_object_code}
    }}
    
    fn get_category(&self) -> Option<String> {{
        return {}
    }}

    {loadmultiplefn}
    {nbdefinedfn}

    {backreferencing}
    {get_referencing}
    {get_relations}
    
    {generate_rust_traits_get_object_iter}",
        match object.get_category() {
            Some(v) => format!("Some(String::from(\"{}\"))", v),
            None => format!("None"),
        },
        loadmultiplefn = loadmultiplefn(object),
        nbdefinedfn = nbdefinedfn(object),
        backreferencing = backreferencing(object, db),
        get_referencing = get_referencing(object, db),
        get_relations = get_relations(object, db),
        generate_rust_traits_get_object_iter = generate_rust_traits_get_object_iter(object)
    )
}

fn get_relations(object: &Object, db: &ObjectDB) -> String {
    let mut code = String::from("fn get_related(
        &self,
        connection: &mut diesel::sqlite::SqliteConnection,
        instance_id: i32,
        related_object: &str,
        relation_object: &str,
        referencing_attribute: &str,
    ) -> Result<Vec<Box<dyn Object>>, bdmg::Error> {
        ");
    let mut nb_relations_found = 0;
    for referencing_name in object.get_referencing_objects() {
        let relation_object = match db.get_object(referencing_name) {
            Some(referencing_object) => referencing_object,
            None => continue,
        };
        let other_object = match relation_object.is_object_relation() {
            Some((source, destination)) => {
                if source == object.get_name() {
                    db.get_object(destination).unwrap()
                } else {
                    db.get_object(source).unwrap()
                }
            },
            None => continue,
        };
        let attribute_name = relation_object.get_relation_attribute(object.get_name()).unwrap().get_name();
        let other_object_name = other_object.get_name();
        let other_table_name = other_object.get_table_name();
        let mut selected_attributes = String::new();
        for at in other_object.get_attributes() {
            selected_attributes = format!("{selected_attributes}                    super::schema::{other_table_name}::{attribute_name},\n", attribute_name = at.get_name());
        }
        
        let relation_table = relation_object.get_table_name();

        code = format!("{code} if relation_object == \"{referencing_name}\"
            && referencing_attribute == \"{attribute_name}\"
            && related_object == \"{other_object_name}\"
        {{
            let sql_result: Vec<super::{other_object_name}> = super::schema::{other_table_name}::dsl::{other_table_name}
                .select((
                    super::schema::{other_table_name}::id,
                    {selected_attributes}
                    super::schema::{other_table_name}::version,
                ))
                .distinct()
                .inner_join(super::schema::{relation_table}::dsl::{relation_table})
                .filter(super::schema::{relation_table}::dsl::{attribute_name}.eq(instance_id))
                .order(super::schema::{other_table_name}::id.asc())
                .load::<super::{other_object_name}>(connection)?;
            let mut result = Vec::<Box<dyn bdmg::Object>>::with_capacity(sql_result.len());
            for other in sql_result {{
                result.push(Box::new(other))
            }}
            Ok(result)
        }} else ");
        nb_relations_found += 1;
    }
    if nb_relations_found > 0 {
        format!("{code} {{ 
            Err(bdmg::Error::ElementNotFound) 
        }}
    }}")
    } else {
        String::from("fn get_related(
            &self,
            _connection: &mut diesel::sqlite::SqliteConnection,
            _instance_id: i32,
            _related_object: &str,
            _relation_object: &str,
            _referencing_attribute: &str,
        ) -> Result<Vec<Box<dyn bdmg::Object>>, bdmg::Error> {
            Err(bdmg::Error::ElementNotFound)
        }")
    }
}

fn get_referencing(object: &Object, db: &ObjectDB) -> String {
    if !object.is_referenced() {
        return String::from("fn get_referencing(
        &self,
        _connection: &mut diesel::sqlite::SqliteConnection,
        _instance_id: i32,
        _ref_table: &str,
        _ref_attribute: &str,
    ) -> Result<Vec<Box<dyn bdmg::Object>>, bdmg::Error> {
        Err(bdmg::Error::ElementNotFound)
    }")
    }
    let mut code = String::from("fn get_referencing(
        &self,
        connection: &mut diesel::sqlite::SqliteConnection,
        instance_id: i32,
        ref_table: &str,
        ref_attribute: &str,
    ) -> Result<Vec<Box<dyn bdmg::Object>>, bdmg::Error> {
        ");
    for referencing_name in object.get_referencing_objects() {
        let referencing_object = match db.get_object(referencing_name) {
            Some(referencing_object) => referencing_object,
            None => continue,
        };
        let attribute_referencing_object =
            match referencing_object.get_relation_attribute(object.get_name()) {
                Some(at) => at.get_name(),
                None => continue,
            };
        let referencing_table_name = referencing_object.get_table_name();
        let mut selected_attributes = String::new();
        for at in referencing_object.get_attributes() {
            selected_attributes = format!("{selected_attributes}                    super::schema::{referencing_table_name}::{attribute_name},\n", attribute_name = at.get_name());
        }
        let branch = format!("if ref_table == \"{referencing_name}\" && ref_attribute == \"{attribute_referencing_object}\" {{
            let sql_result: Vec<super::{referencing_name}> = super::schema::{referencing_table_name}::dsl::{referencing_table_name}
                .select((
                    super::schema::{referencing_table_name}::id,
{selected_attributes}                    super::schema::{referencing_table_name}::version,
                ))
                .filter(super::schema::{referencing_table_name}::{attribute_referencing_object}.eq(instance_id))
                .order(super::schema::{referencing_table_name}::id.asc())
                .load::<super::{referencing_name}>(connection)?;
            let mut result = Vec::<Box<dyn bdmg::Object>>::with_capacity(sql_result.len());
            for other in sql_result {{
                result.push(Box::new(other))
            }}
            Ok(result)
        }} else ");
        code = format!("{code}{branch}");
    }

    format!("{code} {{
            Err(bdmg::Error::ElementNotFound)
        }}
    }}")
}

fn backreferencing(object: &Object, db: &ObjectDB) -> String {
    if !object.is_referenced() {
        return String::from("fn get_back_references(&self) -> Vec<bdmg::BackReference> {
        vec![]
    }")
    }
    let mut code = String::from("fn get_back_references(&self) -> Vec<bdmg::BackReference> {\n        vec![");
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
        code = format!("{code}\n            bdmg::BackReference::new(super::{}::get_object_introspection(), String::from(\"{}\")),",
            referencing.get_name(), attribute_referencing_object);
    }
    format!("{code}\n        ]\n    }}")
}


fn loadmultiplefn(object: &Object) -> String {
    format!(
        "
    ///Load up to a given number of instances, starting with a given id
    fn load_multiple(
        &self,
        from: i32,
        max_count: i32,
        connection: &mut SqliteConnection,
    ) -> Result<Vec<Box<(dyn Object + 'static)>>, bdmg::Error> {{
        let result = {table_name}::dsl::{table_name}
            {select_clause}
            .order({table_name}::id.asc())
            .limit(max_count.into())
            .offset(from.into())
            .load::<{object_name}>(connection)?;
        let mut dyn_objects = Vec::<Box<(dyn Object + 'static)>>::with_capacity(result.len());
        for instance in result {{
            dyn_objects.push(Box::new(instance))
        }}
        Ok(dyn_objects)
    }}",
        object_name = object.get_name(),
        table_name = object.get_table_name(),
        select_clause = super::generate_rust_select_clause(object, 3),
    )
}

fn nbdefinedfn(object: &Object) -> String {
    format!("
    /// Retrieve the number of instances present on database
    fn get_nb_defined(&self, connection: &mut SqliteConnection) -> i64 {{
        match {table_name}::dsl::{table_name}.select(diesel::dsl::count({table_name}::id)).limit(1).get_result::<i64>(connection) {{
            Ok(v)   => v,
            Err(_e) => 0
        }}
    }}",
        table_name = object.get_table_name())
}

fn generate_rust_traits_get_object_iter(object: &Object) -> String {
    format!(
        "
    fn get_objects<'a>(
        &self,
        connection: &'a mut diesel::sqlite::SqliteConnection,
    ) -> ObjectIterator<'a> {{
        let max_id = match {table_name}::dsl::{table_name}.select({table_name}::id)
                         .order({table_name}::id.desc())
                         .limit(1)
                         .get_result::<i32>(connection) {{
            Ok(v) => v,
            Err(_e) => -1
        }};
        ObjectIterator::new(0, max_id, connection, retrieve_next_{object_lowercase}_object)
    }}",
        table_name = object.get_table_name(),
        object_lowercase = object.get_name().to_ascii_lowercase() /* function name */
    )
}

// SQLRepresentation trait

fn generate_sqlrepresentation_traits_impl(object: &Object) -> String {
    format!(
        "impl SqlRepresentation for {} {{\n{}\n{}\n{}\n}}",
        object.get_name(),
        generate_rust_traits_impl_table_name(object),
        generate_rust_traits_impl_attr_names(object),
        generate_rust_traits_impl_boxed_introspection(object)
    )
}

fn generate_rust_traits_impl_table_name(object: &Object) -> String {
    format!(
        "    fn table_name() -> &'static str {{
        \"{}\"
    }}",
        object.get_table_name()
    )
}

fn generate_rust_traits_impl_attr_names(object: &Object) -> String {
    let mut attr_list = String::new();
    let mut separator = String::new();
    for at in object.get_attributes() {
        if !at.is_secret() {
            attr_list = format!("{}{}\"{}\"", attr_list, separator, at.get_name());
            separator = ", ".to_string();
        }
    }
    format!(
        "    fn get_attribute_names() -> &'static [&'static str] {{
        &[{}]
    }}",
        attr_list
    )
}

fn generate_rust_traits_impl_boxed_introspection(object: &Object) -> String {
    format!(
        "    fn get_object_introspection() -> Box<dyn ObjectIntrospection> {{
        Box::new({}ObjectIntrospection{{}})
    }}",
        object.get_name()
    )
}

// Object trait

fn generate_object_traits_impl(object: &Object) -> String {
    format!(
        "impl Object for {object_name} {{
    fn get_id(&self) -> i32 {{
        self.id
    }}
    fn get_version(&self) -> i64 {{
        self.version
    }}
    fn get_attribute(&self, attribute : &str) -> Result<String, String> {{
        {attr_getters}
    }}
    fn set_attribute(&mut self, attribute: &str, _value: &str, _connection: &mut SqliteConnection) -> Result<(), bdmg::Error> {{
        match attribute.as_ref() {{
{attr_setters}            \"id\" => Err(bdmg::Error::ImmutableAttribute(attribute.to_string())),
            \"version\" => Err(bdmg::Error::ImmutableAttribute(attribute.to_string())),
            _ => Err(bdmg::Error::UnknownAttribute(attribute.to_string())),
        }}
    }}
    fn drop(self: Box<Self>, connection: &mut SqliteConnection) -> Result<(), bdmg::Error> {{
        self.delete(connection)
    }}
    fn type_name(&self) -> &'static str {{
        \"{object_name}\"
    }}
}}",
        object_name = object.get_name(),
        attr_getters = generate_traits_impl_object_get_attr(object),
        attr_setters = generate_traits_impl_object_set_attr(object),
    )
}

fn generate_traits_impl_object_get_attr(object: &Object) -> String {
    if !object.has_public_attributes() {
        return String::from("Err(format!(\"Undefined attribute {}\", attribute))");
    }
    let mut matches = String::new();
    for at in object.get_attributes() {
        if !at.is_secret() {
            matches = format!(
                "{}
            \"{}\" => {},",
                matches,
                at.get_name(),
                {
                    match at.get_type() {
                        AttributeType::Mandatory(base_type) => match base_type {
                            BaseAttributeType::Integer => {
                                format!("Ok(format!(\"{{}}\",self.get_{}()))", at.get_name())
                            }
                            BaseAttributeType::String => {
                                format!("Ok(self.get_{}().clone())", at.get_name())
                            }
                            BaseAttributeType::Reference(_r) => {
                                format!("Ok(format!(\"{{}}\", self.{}))", at.get_name())
                            }
                        },
                        AttributeType::Optional(_base_type) => format!(
                            "match &self.{} {{
                Some(v) => Ok(format!(\"({{}})\", v)),
                None => Ok(String::new())
            }}",
                            at.get_name()
                        ),
                    }
                }
            );
        }
    }
    format!("match attribute.as_ref() {{ {matches}
            _ => Err(format!(\"Undefined attribute {{}}\", attribute))
        }}")
}

fn generate_traits_impl_object_set_attr(object: &Object) -> String {
    let mut matches = String::new();
    for at in object.get_attributes() {
        if at.is_mutable() {
            let value_expression = match at.get_type() {
                AttributeType::Mandatory(_) => {
                    format!("_value.parse::<{}>()", super::get_rust_type(at))
                }
                AttributeType::Optional(base) => format!(
                    "bdmg::extract_optional::<{}>(_value)",
                    super::get_base_type(base)
                ),
            };
            let set_expression = match at.get_type().get_base_type() {
                BaseAttributeType::Reference(r) => match at.get_type() {
                    AttributeType::Mandatory(_) => {
                        format!("
                    match {}::load(_connection, v) {{
                        Ok(b) => self.set_{}(&b, _connection),
                        Err(e) => return Err(bdmg::Error::InvalidAttributeValue(format!(\"Unable to load referenced type: {{}}\", e)))
                    }}
               ",
                            r,
                            at.get_name())
                    }
                    AttributeType::Optional(_) => {
                        format!("
                    match v {{
                        None => self.set_{}(None, _connection),
                        Some(idx) => {{
                            match {}::load(_connection, idx) {{
                                Ok(b) => self.set_{}(Some(&b), _connection),
                                Err(e) => return Err(bdmg::Error::InvalidAttributeValue(format!(\"Unable to load referenced type: {{}}\", e)))
                            }}
                        }}
                    }}",
                                at.get_name(),
                                r,
                                at.get_name())
                    }
                },
                _ => format!("self.set_{}(v, _connection)", at.get_name()),
            };
            let branch_content = format!(
                "                let value = {};
                match value {{
                    Ok(v) => {{ {} }},
                    Err(e) => {{ return Err(bdmg::Error::ParsingError(Box::new(e))) }}
                }}
            ",
                value_expression, set_expression
            );
            matches =
                matches + "            \"" + at.get_name() + "\" => {\n" + &branch_content + "},\n"
        } else {
            matches = matches
                + "        \""
                + at.get_name()
                + "\" => Err(bdmg::Error::ImmutableAttribute(attribute.to_string())),\n"
        }
    }
    matches
}

// Object factory
fn generate_object_factory_struct(object: &Object) -> String {
    let mut atdef = String::new();
    for at in object.get_attributes() {
        atdef =
            atdef + "    " + at.get_name() + ": Option<" + &super::get_rust_type(at) + ">,\n";
    }
            
    format!(
        "/// structure containing the different attributes that can be set when constructing an instance of {object_name}\nstruct {object_name}ObjectFactory {{\n{struct_attr}\n}}\n", 
        object_name = object.get_name(), 
        struct_attr = atdef)
}

fn generate_object_factory_traits_impl(object: &Object) -> String {    
    format!(
"impl ObjectFactory for {object_name}ObjectFactory {{
    fn set_attribute(&mut self, attribute_name: &str, attribute_value: &str) -> Result<(), bdmg::Error> {{\n        {setter}\n    }}
    fn create(&mut self, connection: &mut diesel::sqlite::SqliteConnection) -> Result<Box<(dyn Object + 'static)>, bdmg::Error> {{\n{create}\n    }}\n}}",
    object_name = object.get_name(),
    setter = generate_object_factory_traits_impl_setter(object),
    create = generate_object_factory_traits_impl_create(object))
}

fn generate_object_factory_traits_impl_setter(object: &Object) -> String {
    let mut branches = String::new();
    for at in object.get_attributes() {
        let branch_content = match at.get_type().get_base_type() {
            BaseAttributeType::String => {
                match at.get_type() {
                    AttributeType::Mandatory(_) => format!("                self.{} = Some(attribute_value.to_string())\n            ", at.get_name()),
                    AttributeType::Optional(_) => format!("                self.{} = Some(Some(attribute_value.to_string()))\n            ", at.get_name())
                }
            },
            _ => {
                let value_retriever = match at.get_type() {
                    AttributeType::Mandatory(_) => format!("attribute_value.parse::<{}>()", super::get_rust_type(at)),
                    AttributeType::Optional(base) => format!("bdmg::extract_optional::<{}>(attribute_value)", super::get_base_type(base))
                };
                format!(
"                let value = {value};
            match value {{
                Ok(v) => {{ self.{attribute_name} = Some(v) }},
                Err(e) => {{ self.{attribute_name} = None; return Err(bdmg::Error::ParsingError(Box::new(e))) }}
            }}
        ",
                        value = value_retriever,
                        attribute_name = at.get_name())
            }
        };
        branches =
            branches + "            \"" + at.get_name() + "\" => {\n" + &branch_content + "},\n"
    }

    format!("match attribute_name {{
        \"id\" => {{}},
        \"version\" => {{}},
{}            _ => {{ return Err(bdmg::Error::UnknownAttribute(attribute_name.to_string())); }}
      }}
      Ok(())", branches)
}

fn generate_object_factory_traits_impl_create(object: &Object) -> String {
    let mut init = String::new();
    let mut constr = String::new();
    for at in object.get_attributes() {
        let not_found_case = match at.get_type() {
            AttributeType::Mandatory(_) => {
                format!("return Err(bdmg::Error::MissingMandatoryAttribute(String::from(\"{attribute_name}\")))", attribute_name = at.get_name())
            }
            AttributeType::Optional(_) => {
                format!("None")
            }
        };

        let found_case = match at.get_reference() {
            None => format!("Some(_) => self.{}.take().unwrap()", at.get_name()),
            Some(referenced_type) => match at.get_type() {
                AttributeType::Mandatory(_v) => {
                    format!(
                        "Some(_) => {}::load(connection, self.{}.take().unwrap())?",
                        referenced_type,
                        at.get_name()
                    )
                }
                AttributeType::Optional(_v) => {
                    format!("Some(Some(_)) => Some({}::load(connection, self.{}.take().unwrap().unwrap())?)", referenced_type, at.get_name())
                }
            },
        };

        init = init
            + &format!(
                "        let a_{attribute_name} = match self.{attribute_name} {{
            {case_some},
            _ => {case_none}
        }};\n",
                attribute_name = at.get_name(),
                case_some = found_case,
                case_none = not_found_case
            );

        match at.get_reference() {
            None => constr = constr + &format!(", a_{}", at.get_name()),
            Some(_v) => match at.get_type() {
                AttributeType::Optional(_) => {
                    constr = constr + &format!(", a_{}.as_ref()", at.get_name())
                }
                AttributeType::Mandatory(_) => {
                    constr = constr + &format!(", &a_{}", at.get_name())
                }
            },
        }
    }
    format!(
        "{}        let instance = match {}::new(connection{}) {{
            Err(e) => return Err(e),
            Ok(v) => v 
        }};
        Ok(Box::new(instance))",
        init, object.get_name(), constr
    )
}
