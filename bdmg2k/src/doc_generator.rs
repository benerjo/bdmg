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
use crate::{BaseAttributeType, Error};

use std::collections::BTreeMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

fn get_md_file(dir: &PathBuf, doc_name: &str) -> Result<(File, String), Error> {
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
    destination.set_extension("md");

    let file_name = match destination.as_path().to_str() {
        Some(pth) => String::from(pth),
        None => format!("{}.md", doc_name),
    };

    match File::create(destination.as_path()) {
        Err(_e) => {
            return Err(Error::UnableToCreateFile {
                file: file_name,
            })
        }
        Ok(f) => Ok((f, file_name)),
    }
}

fn get_dot_file(dir: &PathBuf, doc_name: &str) -> Result<(File, String), Error> {
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
    destination.set_extension("dot");

    let filename = match destination.as_path().to_str() {
        Some(pth) => String::from(pth),
        None => format!("{}.dot", doc_name),
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

fn create_category_mapping(object_db: &ObjectDB) -> BTreeMap<String, Vec<&Object>> {
    let mut result: BTreeMap<String, Vec<&Object>> = BTreeMap::new();
    for object in object_db.get_objects() {
        let category = match object.get_category() {
            Some(c) => c.clone(),
            None => String::new(),
        };
        match result.get_mut(&category) {
            Some(v) => v.push(&object),
            None => {
                result.insert(category, vec![&object]);
            }
        }
    }
    //Sort the vector of objects by their name
    for (_category_name, object_vector) in &mut result {
        object_vector.sort_by(|object_a, object_b| object_a.get_name().cmp(object_b.get_name()))
    }
    result
}

fn get_base_attr_type(atype: &BaseAttributeType) -> String {
    match atype {
        BaseAttributeType::Integer => String::from("integer"),
        BaseAttributeType::String => String::from("string"),
        BaseAttributeType::Reference(other) => {
            format!(
                "reference to <a href=\"#{other}\">{other}</a>",
                other = other
            )
        }
    }
}

fn get_object_doc(object: &Object) -> String {
    let mut attribute_desc =
        String::from("<tr><th>Name</th><th>Type</th><th>Explanation</th></tr>\n\n");
    for at in object.get_attributes() {
        let attribute_comment = match at.get_comment() {
            Some(c) => c.clone(),
            None => String::new(),
        };

        let secret = if at.is_secret() {
            String::from("secret ")
        } else {
            String::new()
        };

        let indexable = if at.is_indexable() {
            String::from("unique ")
        } else {
            String::new()
        };

        let optional = if at.is_optional() {
            String::from("optional ")
        } else {
            String::new()
        };

        let immutable = if at.is_mutable() {
            String::new()
        } else {
            String::from("immutable ")
        };

        let atype = format!(
            "{secret}{unique}{optional}{immutable}{base_type}",
            secret = secret,
            unique = indexable,
            optional = optional,
            immutable = immutable,
            base_type = get_base_attr_type(at.get_type().get_base_type())
        );

        attribute_desc = format!(
            "{desc}\n<tr><td>{name}</td><td>{atype}</td><td>{expl}</td></tr>",
            desc = attribute_desc,
            name = at.get_name(),
            expl = attribute_comment,
            atype = atype
        );
    }

    let desc = match object.get_description() {
        Some(d) => d.clone(),
        None => String::new(),
    };
    format!(
        "{desc}\n\n*table name*: {table_name}\n\n<table>{attributes}\n</table>",
        desc = desc,
        table_name = object.get_table_name(),
        attributes = attribute_desc
    )
}

fn get_dot_node_arcs(object: &Object) -> (String, String) {
    //if an object is a relation but is referenced anywhere, then we must draw it nevertheless
    match (object.is_referenced(), object.is_object_relation()) {
        (false, Some((from, to))) => (
            String::new(),
            format!(
                "\n    \"{from}\":n -> \"{to}\":n [label=\"{object_name}\", dir=\"both\"]",
                from = from,
                to = to,
                object_name = object.get_name(),
            ),
        ),
        _ => {
            let mut label = format!("<<TABLE BORDER=\"0\" CELLBORDER=\"1\" CELLSPACING=\"0\"> <TR><TD PORT=\"n\">{object_name}</TD></TR>", object_name = object.get_name());
            let mut arcs = String::new();
            let mut attribute_index = 0;
            for at in object.get_attributes() {
                label = format!(
                    "{label}<TR><TD PORT=\"f{index}\">{at_name}</TD></TR>",
                    label = label,
                    index = attribute_index,
                    at_name = at.get_name()
                );
                match at.get_reference() {
                    Some(referenced_object) => {
                        let style = if at.is_optional() {
                            String::from(" [style=dashed]")
                        } else {
                            String::new()
                        };
                        arcs = format!(
                            "{prev}\n    \"{object_name}\":f{index} -> \"{reference}\":n{style}",
                            prev = arcs,
                            object_name = object.get_name(),
                            index = attribute_index,
                            reference = referenced_object,
                            style = style
                        )
                    }
                    None => {}
                }
                attribute_index += 1;
            }
            (
                format!(
                    "\"{object_name}\" [shape=\"plaintext\", label={label}</TABLE>>]",
                    object_name = object.get_name(),
                    label = label
                ),
                arcs,
            )
        }
    }
}

pub fn write_doc(object_db: &ObjectDB, destination: &str, doc_name: &str) -> Result<(), Error> {
    let pbuf = PathBuf::from(destination);

    if !pbuf.exists() {
        match std::fs::create_dir(&pbuf) {
            Ok(()) => {},
            Err(e) => return Err(Error::UnableToCreateOutputDirectory { destination: destination.to_string(), error: e }),
        }
    }

    if !pbuf.is_dir() {
        return Err(Error::DestinationIsNotDirectory {
            destination: String::from(destination),
        });
    }

    let object_mapping = create_category_mapping(object_db);

    let mut toc = String::from("");
    let mut content = String::from("");
    let mut cat_index = 1;

    let mut subgraphs = String::from("");
    let mut all_arcs = String::from("");

    for (cat, objects) in &object_mapping {
        toc = format!(
            "{toc}\n{index}. [{category_name}](#{category_name})",
            toc = toc,
            index = cat_index,
            category_name = cat
        );
        content = format!(
            "{content}\n\n<a name=\"{category_name}\"></a>\n##{index}. {category_name}",
            content = content,
            index = cat_index,
            category_name = cat
        );

        let mut subgraph_nodes = String::new();
        let mut subgraph_arcs = String::new();
        let mut obj_index = 1;
        for obj in objects {
            toc = format!(
                "{toc}\n    {index}. [{object_name}](#{object_name})",
                toc = toc,
                index = obj_index,
                object_name = obj.get_name()
            );

            content = format!(
                "{content}\n\n<a name=\"{object_name}\"></a>\n##{cat_index}.{index}. {object_name}\n{obj_doc}",
                content = content,
                cat_index = cat_index,
                index = obj_index,
                object_name = obj.get_name(),
                obj_doc = get_object_doc(obj),
            );

            let (node, arcs) = get_dot_node_arcs(obj);
            subgraph_nodes = format!("{prev}\n    {new}", prev = subgraph_nodes, new = node);
            subgraph_arcs = format!("{prev}{new}", prev = subgraph_arcs, new = arcs);

            obj_index += 1;
        }

        subgraphs = format!(
            "{prev}\n  subgraph cluster_{cat_name} {{\n    label=\"{cat_name}\";\n    color=lightgrey;\n    {nodes}\n  }}", 
            prev = subgraphs,
            cat_name = cat,
            nodes = subgraph_nodes
        );

        all_arcs = format!(
            "{prev}\n    {new_arcs}",
            prev = all_arcs,
            new_arcs = subgraph_arcs
        );

        cat_index += 1;
    }

    let diagram = format!("<a name=\"Diagram\"></a>\n## Diagram\n\nThe following diagram shows the relation between the different element of the data model.\n\n![Data model diagram](./{doc_name}.svg)", doc_name = doc_name);
    content = format!("{}\n{}", content, diagram);
    toc = format!(
        "{toc}\n{index}. [Diagram](#Diagram)",
        toc = toc,
        index = cat_index
    );

    let markdown_content = format!("#Data Model\n##Table of content\n{}\n{}", toc, content);

    let (mut markdown_file, md_filename) = get_md_file(&pbuf, doc_name)?;

    match markdown_file.write(markdown_content.as_bytes()) {
        Err(_e) => {
            return Err(Error::UnableToWriteToFile {
                file: md_filename,
                content: markdown_content,
            })
        }
        Ok(size_written) => {
            if size_written != markdown_content.as_bytes().len() {
                return Err(Error::UnableToWriteToFile {
                    file: md_filename,
                    content: markdown_content,
                });
            }
        }
    };

    let dot_content = format!(
        "digraph g{{\n  graph [ rankdir=\"LR\" ];{subgraphs}\n{arcs}}}",
        subgraphs = subgraphs,
        arcs = all_arcs,
    );
    let (mut dot_file, dot_filename) = get_dot_file(&pbuf, doc_name)?;
    match dot_file.write(dot_content.as_bytes()) {
        Err(_e) => {
            return Err(Error::UnableToWriteToFile {
                file: dot_filename,
                content: dot_content,
            })
        }
        Ok(size_written) => {
            if size_written != dot_content.as_bytes().len() {
                return Err(Error::UnableToWriteToFile {
                    file: dot_filename,
                    content: dot_content,
                });
            }
        }
    };

    return Ok(());
}
