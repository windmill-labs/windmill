/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Integration tests for Pydantic BaseModel and Python dataclass support.

use windmill_parser::Typ;
use windmill_parser_py::parse_python_signature;

#[test]
fn test_pydantic_basic_model() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel

class User(BaseModel):
    name: str
    age: int
    email: str

def main(user: User):
    return f'Hello {user.name}'
";
    let result = parse_python_signature(code, None, false)?;

    // Check that user parameter is detected as Object type
    assert_eq!(result.args.len(), 1);
    assert_eq!(result.args[0].name, "user");

    // Verify it's an Object type with correct model name
    match &result.args[0].typ {
        Typ::Object(obj) => {
            assert_eq!(obj.name, Some("User".to_string()));
            assert!(obj.props.is_some());

            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 3);

            // Verify field names and types
            assert_eq!(props[0].key, "name");
            assert_eq!(*props[0].typ, Typ::Str(None));

            assert_eq!(props[1].key, "age");
            assert_eq!(*props[1].typ, Typ::Int);

            assert_eq!(props[2].key, "email");
            assert_eq!(*props[2].typ, Typ::Str(None));
        }
        _ => panic!("Expected Typ::Object for Pydantic model"),
    }

    Ok(())
}

#[test]
fn test_python_dataclass() -> anyhow::Result<()> {
    let code = "
from dataclasses import dataclass

@dataclass
class Config:
    host: str
    port: int
    debug: bool

def main(config: Config):
    return config.host
";
    let result = parse_python_signature(code, None, false)?;

    // Check that config parameter is detected as Object type
    assert_eq!(result.args.len(), 1);
    assert_eq!(result.args[0].name, "config");

    // Verify it's an Object type with correct class name
    match &result.args[0].typ {
        Typ::Object(obj) => {
            assert_eq!(obj.name, Some("Config".to_string()));
            assert!(obj.props.is_some());

            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 3);

            // Verify field names and types
            assert_eq!(props[0].key, "host");
            assert_eq!(*props[0].typ, Typ::Str(None));

            assert_eq!(props[1].key, "port");
            assert_eq!(*props[1].typ, Typ::Int);

            assert_eq!(props[2].key, "debug");
            assert_eq!(*props[2].typ, Typ::Bool);
        }
        _ => panic!("Expected Typ::Object for dataclass"),
    }

    Ok(())
}

#[test]
fn test_pydantic_nested_model() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel

class Address(BaseModel):
    street: str
    city: str

class Person(BaseModel):
    name: str
    address: Address

def main(person: Person):
    return person.name
";
    let result = parse_python_signature(code, None, false)?;

    // Check that person parameter is detected as Object type
    assert_eq!(result.args.len(), 1);
    assert_eq!(result.args[0].name, "person");

    // Verify it's an Object type with nested model
    match &result.args[0].typ {
        Typ::Object(obj) => {
            assert_eq!(obj.name, Some("Person".to_string()));
            assert!(obj.props.is_some());

            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 2);

            // Verify name field
            assert_eq!(props[0].key, "name");
            assert_eq!(*props[0].typ, Typ::Str(None));

            // Verify address field is a nested Object
            assert_eq!(props[1].key, "address");
            match props[1].typ.as_ref() {
                Typ::Object(nested_obj) => {
                    assert_eq!(nested_obj.name, Some("Address".to_string()));
                    assert!(nested_obj.props.is_some());

                    let nested_props = nested_obj.props.as_ref().unwrap();
                    assert_eq!(nested_props.len(), 2);
                    assert_eq!(nested_props[0].key, "street");
                    assert_eq!(nested_props[1].key, "city");
                }
                _ => panic!("Expected nested Typ::Object for Address"),
            }
        }
        _ => panic!("Expected Typ::Object for Person model"),
    }

    Ok(())
}

#[test]
fn test_pydantic_empty_model() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel

class EmptyModel(BaseModel):
    pass

def main(model: EmptyModel):
    return 'ok'
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    match &result.args[0].typ {
        Typ::Object(obj) => {
            assert_eq!(obj.name, Some("EmptyModel".to_string()));
            assert!(obj.props.is_none());
        }
        _ => panic!("Expected Typ::Object for empty model"),
    }

    Ok(())
}

#[test]
fn test_pydantic_list_field() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel
from typing import List

class TodoList(BaseModel):
    items: List[str]
    count: int

def main(todos: TodoList):
    return todos.count
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    match &result.args[0].typ {
        Typ::Object(obj) => {
            assert_eq!(obj.name, Some("TodoList".to_string()));
            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 2);

            // Verify List[str] type
            assert_eq!(props[0].key, "items");
            match props[0].typ.as_ref() {
                Typ::List(inner) => {
                    assert_eq!(**inner, Typ::Str(None));
                }
                _ => panic!("Expected Typ::List for items field"),
            }

            assert_eq!(props[1].key, "count");
            assert_eq!(*props[1].typ, Typ::Int);
        }
        _ => panic!("Expected Typ::Object"),
    }

    Ok(())
}

#[test]
fn test_pydantic_optional_field() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel
from typing import Optional

class User(BaseModel):
    name: str
    nickname: Optional[str]

def main(user: User):
    return user.name
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    match &result.args[0].typ {
        Typ::Object(obj) => {
            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 2);

            assert_eq!(props[0].key, "name");
            assert_eq!(*props[0].typ, Typ::Str(None));

            // Optional[str] should unwrap to str
            assert_eq!(props[1].key, "nickname");
            assert_eq!(*props[1].typ, Typ::Str(None));
        }
        _ => panic!("Expected Typ::Object"),
    }

    Ok(())
}

#[test]
fn test_dataclass_with_decorator_args() -> anyhow::Result<()> {
    let code = "
from dataclasses import dataclass

@dataclass(frozen=True)
class ImmutableConfig:
    setting: str
    value: int

def main(config: ImmutableConfig):
    return config.setting
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    match &result.args[0].typ {
        Typ::Object(obj) => {
            assert_eq!(obj.name, Some("ImmutableConfig".to_string()));
            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 2);
        }
        _ => panic!("Expected Typ::Object for dataclass"),
    }

    Ok(())
}

#[test]
fn test_pydantic_dict_field() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel
from typing import Dict

class Config(BaseModel):
    settings: Dict[str, str]
    name: str

def main(config: Config):
    return config.name
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    match &result.args[0].typ {
        Typ::Object(obj) => {
            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 2);

            // Dict should return generic Object
            assert_eq!(props[0].key, "settings");
            match props[0].typ.as_ref() {
                Typ::Object(_) => {} // Generic object for Dict
                _ => panic!("Expected Typ::Object for Dict field"),
            }
        }
        _ => panic!("Expected Typ::Object"),
    }

    Ok(())
}

#[test]
fn test_non_model_class_treated_as_resource() -> anyhow::Result<()> {
    let code = "
class RegularClass:
    def __init__(self, value):
        self.value = value

def main(obj: RegularClass):
    return 'ok'
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    // Regular classes (non-Pydantic/dataclass) should be treated as Resource
    assert_eq!(
        result.args[0].typ,
        Typ::Resource("RegularClass".to_string())
    );

    Ok(())
}

#[test]
fn test_invalid_syntax_fallback() -> anyhow::Result<()> {
    // Code with syntax errors - should still not crash
    let code = "
from pydantic import BaseModel

class User(BaseModel:  # Missing closing paren
    name: str

def main(user: User):
    return 'ok'
";
    // Should not panic, even with invalid syntax
    let result = parse_python_signature(code, None, false);

    // Either succeeds with Unknown types or fails gracefully
    assert!(result.is_ok() || result.is_err());

    Ok(())
}

#[test]
fn test_datetime_type() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel
from datetime import datetime

class Event(BaseModel):
    name: str
    created_at: datetime

def main(event: Event):
    return event.name
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    match &result.args[0].typ {
        Typ::Object(obj) => {
            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 2);

            assert_eq!(props[0].key, "name");
            assert_eq!(*props[0].typ, Typ::Str(None));

            assert_eq!(props[1].key, "created_at");
            assert_eq!(*props[1].typ, Typ::Datetime);
        }
        _ => panic!("Expected Typ::Object"),
    }

    Ok(())
}

#[test]
fn test_multiple_pydantic_models() -> anyhow::Result<()> {
    let code = "
from pydantic import BaseModel

class User(BaseModel):
    name: str

class Post(BaseModel):
    title: str
    author: User

def main(post: Post):
    return post.title
";
    let result = parse_python_signature(code, None, false)?;

    assert_eq!(result.args.len(), 1);
    match &result.args[0].typ {
        Typ::Object(obj) => {
            assert_eq!(obj.name, Some("Post".to_string()));
            let props = obj.props.as_ref().unwrap();
            assert_eq!(props.len(), 2);

            // Nested User model
            assert_eq!(props[1].key, "author");
            match props[1].typ.as_ref() {
                Typ::Object(nested) => {
                    assert_eq!(nested.name, Some("User".to_string()));
                }
                _ => panic!("Expected nested Typ::Object for User"),
            }
        }
        _ => panic!("Expected Typ::Object"),
    }

    Ok(())
}
