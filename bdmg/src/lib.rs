/*
    Copyright 2020 benerjo

    This file is part of bdmg.

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
extern crate diesel;
#[macro_use]
extern crate serde_derive;

///Trait to provide the name of the table that holds the different
///records for an object
pub trait SqlRepresentation {
    ///Retrieve the name of the table that contain the object
    fn table_name() -> &'static str;
    ///Retrieve the list of attributes that are defined in the object
    fn get_attribute_names() -> &'static [&'static str];
    ///Retrieve the ObjectIntrospection trait that describe the object
    fn get_object_introspection() -> Box<dyn ObjectIntrospection>;
}

///This struct is providing an iterator interface to some objects
///The return type of the iterator is a result containing a pointer to the trait Object.
///The idea is that the iteration loads only one object at a time from the database.
pub struct ObjectIterator<'a> {
    ///The next id to be loaded. Note that this id might be deleted between being set
    ///and when we want to load it. Therefore, extra caution needs to be taken
    next_id: i32,
    ///The last id that the iterator should provide. Note that if new items are added
    ///to the database, those won't be seen. And, as for next_id, it is possible that
    ///the element with id last_id has been deleted in the mean time
    last_id: i32,
    retriever: fn(
        i32,
        i32,
        &mut diesel::sqlite::SqliteConnection,
    ) -> (i32, Option<Result<Box<(dyn Object + 'static)>, String>>),
    ///The connection that will be used to the database
    connection: &'a mut diesel::sqlite::SqliteConnection,
}

impl<'a> ObjectIterator<'a> {
    ///Create a new object iterator.
    ///This function should provide all accessible records with id's between
    ///first_id to final_id (included). To retrieve the objects, the object
    ///iterator will use the function retrieval_function to do so.
    pub fn new(
        first_id: i32,
        final_id: i32,
        connect: &'a mut diesel::sqlite::SqliteConnection,
        retrieval_function: fn(
            next_id: i32,
            last_id: i32,
            connection: &mut diesel::sqlite::SqliteConnection,
        )
            -> (i32, Option<Result<Box<(dyn Object + 'static)>, String>>),
    ) -> ObjectIterator<'a> {
        ObjectIterator {
            next_id: first_id,
            last_id: final_id,
            connection: connect,
            retriever: retrieval_function,
        }
    }
}

impl<'a> Iterator for ObjectIterator<'a> {
    type Item = Result<Box<(dyn Object + 'static)>, String>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next_id > self.last_id {
            None
        } else {
            let retrieved = (self.retriever)(self.next_id, self.last_id, &mut self.connection);
            self.next_id = retrieved.0;
            retrieved.1
        }
    }
}

///Trait used to build an object on an abstract manner.
pub trait ObjectFactory {
    ///set the value of an attribute
    fn set_attribute(&mut self, attribute_name: &str, attribute_value: &str) -> Result<(), Error>;
    ///create the object and consume the factory, resetting it to default
    fn create(
        &mut self,
        connection: &mut diesel::sqlite::SqliteConnection,
    ) -> Result<Box<(dyn Object + 'static)>, Error>;
}

///Enumeration to represent all potential types that can be provided
#[derive(Serialize, Debug)]
pub enum AttributeType {
    ///An integer type
    Integer,
    ///A string type
    String,
    ///A reference to some other object (value of the related string)
    Reference(String),
}

///Structure to have introspection about attribute of objects
#[derive(Serialize, Debug)]
pub struct Attribute {
    name: String,
    kind: AttributeType,
    optional: bool,
    mutable: bool,
}

#[derive(Debug)]
pub enum Error {
    /// Element not found
    ElementNotFound,
    InternalError(diesel::result::Error),
    ParsingError(Box<dyn std::error::Error>),
    UnknownAttribute(String),
    ImmutableAttribute(String),
    InvalidAttributeValue(String),
    MissingMandatoryAttribute(String),
    UnableToRetrieveIdentifierForTable(String),
    UnableToCreateRecord(String),
    InvalidVersion,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ElementNotFound => write!(f, "Unable to find the requested element"),
            Error::InternalError(e) => write!(f, "Unmanaged database error: {}", e),
            Error::ParsingError(err) => write!(f, "Unable to parse value, {}", err),
            Error::UnknownAttribute(attribute_name) => {
                write!(f, "Attribute '{}' is not known", attribute_name)
            }
            Error::ImmutableAttribute(attribute_name) => {
                write!(f, "Attribute '{}' is not mutable", attribute_name)
            }
            Error::InvalidAttributeValue(msg) => write!(f, "Invalid attribute value {}", msg),
            Error::MissingMandatoryAttribute(attr_name) => {
                write!(f, "Mandatory attribute '{}' not set", attr_name)
            }
            Error::UnableToRetrieveIdentifierForTable(table_name) => write!(
                f,
                "Unable to retrieve an identifier for table '{}'",
                table_name
            ),
            Error::UnableToCreateRecord(record_type) => {
                write!(f, "Unable to create a record of type '{}'", record_type)
            }
            Error::InvalidVersion => write!(f, "The requested version of the object is not valid"),
        }
    }
}

impl std::error::Error for Error {}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        Error::InternalError(error)
    }
}

impl Attribute {
    pub fn new(name: String, kind: AttributeType, optional: bool, mutable: bool) -> Attribute {
        Attribute {
            name: name,
            kind: kind,
            optional: optional,
            mutable: mutable,
        }
    }

    ///Retrieve the name of the attribute
    pub fn get_name(&self) -> &String {
        &self.name
    }

    ///Retrieve the kind of this attribute
    pub fn get_kind(&self) -> &AttributeType {
        &self.kind
    }

    ///Check if the attribute is optional
    pub fn is_optional(&self) -> bool {
        self.optional
    }

    ///Check if the attribute can be modified
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }

    ///Check if this attribute is a reference
    pub fn is_reference(&self) -> bool {
        match self.kind {
            AttributeType::Reference(_) => true,
            _ => false,
        }
    }
}

pub struct BackReference {
    referencing_object: Box<dyn ObjectIntrospection>,
    referencing_attribute: String,
}

impl BackReference {
    ///Create a new BackReference object
    pub fn new(
        referencing_object: Box<dyn ObjectIntrospection>,
        referencing_attribute: String,
    ) -> BackReference {
        BackReference {
            referencing_object,
            referencing_attribute,
        }
    }

    ///Retrieve the name of the object referencing an instance
    pub fn referencing_object(&self) -> &dyn ObjectIntrospection {
        self.referencing_object.as_ref()
    }

    ///Retrieve teh name of the attribute containing a reference to the instance
    pub fn referencing_attribute(&self) -> &str {
        self.referencing_attribute.as_ref()
    }
}

///Trait used to represent an object itself: its name and list of attributes
pub trait ObjectIntrospection {
    ///Retrieve the vector containing all attributes
    fn get_attribute_names(&self) -> Vec<String>;
    ///Retrieve the name of the object
    fn get_object_name(&self) -> String;
    ///Retrieve the iterator to the objects. Objects are loaded one at a time
    fn get_objects<'a>(
        &self,
        connection: &'a mut diesel::sqlite::SqliteConnection,
    ) -> ObjectIterator<'a>;
    ///Retrieve the description of the attributes
    fn get_attributes(&self) -> Vec<Attribute>;
    ///Retrieve the category of the object
    fn get_category(&self) -> Option<String>;
    ///Generate an object factory to create a new object instance
    fn create_factory<'a>(&self) -> Box<(dyn ObjectFactory + 'static)>;
    ///Retrieve the object based on its id and version
    fn get_object(
        &self,
        connection: &mut diesel::sqlite::SqliteConnection,
        id: i32,
        version: Option<i64>,
    ) -> Result<Box<(dyn Object + 'static)>, Error>;
    ///Retrieve the current number of instances
    fn get_nb_defined(&self, connection: &mut diesel::sqlite::SqliteConnection) -> i64;
    ///Load multiple instances, from a given id with a maximum number of instances
    fn load_multiple(
        &self,
        from: i32,
        max_count: i32,
        connection: &mut diesel::sqlite::SqliteConnection,
    ) -> Result<Vec<Box<(dyn Object + 'static)>>, Error>;
    ///Retrieve the list of back references
    fn get_back_references(&self) -> Vec<BackReference>;
    ///Retrieve the list of objects referencing the instance with the given id
    /// A reference is an object for which the current instance contains a
    /// reference to
    fn get_referencing(
        &self,
        connection: &mut diesel::sqlite::SqliteConnection,
        instance_id: i32,
        ref_table: &str,
        ref_attribute: &str,
    ) -> Result<Vec<Box<dyn Object>>, Error>;
    ///Retrieve the list of instances related
    /// Let there be 3 objects: A, B and C. C contains only references to the id's
    /// of A and B through the attribute aid and bid. C is a relation object, allowing
    ///  to relate multiple A for a given B, and multiple B for a given A.
    /// In this example, if we want all B's related to a given A through the relation C, we
    /// would call this method dyn_object_a.get_related(&mut connection, a.get_id(), "B", "C", "aid")
    fn get_related(
        &self,
        connection: &mut diesel::sqlite::SqliteConnection,
        instance_id: i32,
        related_object: &str,
        relation_object: &str,
        referencing_attribute: &str,
    ) -> Result<Vec<Box<dyn Object>>, Error>;
}

///Common trait to all objects
pub trait Object {
    ///Retrieve the type name of this object
    fn type_name(&self) -> &'static str;
    ///Retrieve the identifier of the object
    fn get_id(&self) -> i32;
    ///Retrieve the version of the object
    fn get_version(&self) -> i64;
    ///Retrieve the string representation of an attribute.
    ///The result is contained in an optional even if the attribute must always be set.
    ///If an error occurs, the error message is given in the result.
    fn get_attribute(&self, attribute: &str) -> Result<String, String>;
    ///Set the value of an attribute
    fn set_attribute(
        &mut self,
        attribute: &str,
        value: &str,
        connection: &mut diesel::sqlite::SqliteConnection,
    ) -> Result<(), Error>;
    ///Delete the instance
    fn drop(
        self: Box<Self>,
        connection: &mut diesel::sqlite::SqliteConnection,
    ) -> Result<(), Error>;
}

/// Enumeration to represent the error that might happen when trying to convert
/// an optional value represented as string to the rust typed representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseOptionalError<ParseError>
where
    ParseError: std::error::Error,
{
    MissingOpenParenthesis,
    MissingCloseParenthesis,
    ParsingError(ParseError),
}

impl<ParseError> std::fmt::Display for ParseOptionalError<ParseError>
where
    ParseError: std::error::Error,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseOptionalError::MissingOpenParenthesis => write!(f, "Missing opening parenthesis"),
            ParseOptionalError::MissingCloseParenthesis => write!(f, "Missing closing parenthesis"),
            ParseOptionalError::ParsingError(msg) => {
                write!(f, "Unable to parse the content: {}", msg)
            }
        }
    }
}

impl<ParseError> std::error::Error for ParseOptionalError<ParseError> where
    ParseError: std::error::Error
{
}

impl<ParseError> From<ParseError> for ParseOptionalError<ParseError>
where
    ParseError: std::error::Error,
{
    fn from(error: ParseError) -> ParseOptionalError<ParseError> {
        ParseOptionalError::ParsingError(error)
    }
}

/// Convert a string into an optional value. The format of the string should either be
/// * empty to represent an empty optional
/// * (value) where value represent the value that will be stored in the optional.
/// The type stored in the optional should be implementing the trait std::str::FromStr as
/// it is used to convert the string into the result.
pub fn extract_optional<T: std::str::FromStr>(
    s: &str,
) -> Result<Option<T>, ParseOptionalError<<T as std::str::FromStr>::Err>>
where
    <T as std::str::FromStr>::Err: std::error::Error,
{
    if s.is_empty() {
        return Ok(None);
    }

    if !s.starts_with('(') {
        return Err(ParseOptionalError::MissingCloseParenthesis);
    }
    if !s.ends_with(')') {
        return Err(ParseOptionalError::MissingCloseParenthesis);
    }

    let substring: &str = match s.get(1..s.len() - 1) {
        Some(substr) => substr,
        None => {
            return Err(ParseOptionalError::MissingCloseParenthesis);
        }
    };

    Ok(Some(substring.parse::<T>()?))
}

///Serde visitor used to deserialize object id
pub struct ObjectIdVisitor;
impl<'de> serde::de::Visitor<'de> for ObjectIdVisitor {
    type Value = i64;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Object Id")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v < i64::MAX as i64 && v > i64::MIN as i64 {
            Ok(v as i64)
        } else {
            Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::Signed(v),
                &self,
            ))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v < i64::MAX as u64 {
            Ok(v as i64)
        } else {
            Err(serde::de::Error::invalid_type(
                serde::de::Unexpected::Unsigned(v),
                &self,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::extract_optional;

    #[test]
    fn simple_integer() {
        assert_eq!(Ok(Some(32)), extract_optional("(32)"));
    }

    #[test]
    fn negative_integer() {
        assert_eq!(Ok(Some(-32)), extract_optional("(-32)"));
    }

    #[test]
    fn simple_string() {
        assert_eq!(Ok(Some(String::from("hello"))), extract_optional("(hello)"));
    }

    #[test]
    fn no_value() {
        assert_eq!(Ok(None), extract_optional::<i64>(""));
    }

    #[test]
    fn empty_optional() {
        assert!(extract_optional::<i64>("()").is_err());
    }

    #[test]
    fn missing_open() {
        assert!(extract_optional::<i64>("32)").is_err());
    }

    #[test]
    fn missing_close() {
        assert!(extract_optional::<i64>("(32").is_err());
    }

    #[test]
    fn missing_both() {
        assert!(extract_optional::<i64>("32").is_err());
    }
}
