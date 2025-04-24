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

pub fn generate_rust_free_functions(object: &Object) -> String {
    format!("\n///function used in the ObjectIterator to retrieve the next instance
fn retrieve_next_{object_lowercase}_object<'a>(
    current_id: i32,
    last_id: i32,
    connection: &'a mut diesel::sqlite::SqliteConnection,
) -> (i32, Option<Result<Box<(dyn Object + 'static)>, String>>) {{
    if last_id < current_id {{
        return (current_id, None);
    }}
    let loaded_result = {object_name}::load(connection, current_id);
    let next_id = match {table_name}::dsl::{table_name}.select({table_name}::id)
                         .filter({table_name}::id.gt(current_id))
                         .order({table_name}::id.asc())
                         .limit(1)
                         .get_result::<i32>(connection) {{
        Ok(v) => v,
        Err(diesel::result::Error::NotFound) => last_id + 1,
        Err(e) => {{
            return (current_id+1, Some(Err(format!(\"Unable to retrieve the next id for {object_name}:\\n{{}}\", e))));
        }}
    }};
    match loaded_result {{
        Ok(v) => (next_id, Some(Ok(Box::new(v)))),
        Err(e) => match retrieve_next_{object_lowercase}_object(next_id, last_id, connection) {{
            (_, Some(Err(ee))) => (last_id + 1, Some(Err(format!(\"Unable to load the next {object_name}: {{}}\\n{{}}\", e, ee)))),
            (_, None) => (last_id + 1, None),
            (future_id, Some(Ok(v))) => (future_id, Some(Ok(v))),
        }}
    }}
}}",
        object_lowercase =object.get_name().to_ascii_lowercase(), 
        object_name = object.get_name(),
        table_name = object.get_table_name()
    )
}