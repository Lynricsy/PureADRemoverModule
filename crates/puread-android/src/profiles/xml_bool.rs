use std::borrow::Cow;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, Writer};

use crate::profiles::ProfileError;

pub(super) fn read_bool_value(
    content: &[u8],
    key: &str,
    path: &Path,
) -> Result<bool, ProfileError> {
    let mut reader = Reader::from_reader(content);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(event) | Event::Start(event)) if is_boolean(&event) => {
                if bool_name_matches(&event, key)? {
                    return bool_value(&event);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(source) => return Err(ProfileError::xml(path.to_path_buf(), source.to_string())),
        }
        buf.clear();
    }
    Err(ProfileError::xml(
        path.to_path_buf(),
        format!("boolean key not found: {key}"),
    ))
}

pub(super) fn rewrite_bool_value(
    content: &[u8],
    key: &str,
    value: bool,
    path: &Path,
) -> Result<Vec<u8>, ProfileError> {
    let mut reader = Reader::from_reader(content);
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(event)) if is_boolean(&event) && bool_name_matches(&event, key)? => {
                writer
                    .write_event(Event::Empty(rewrite_event(&event, value)?))
                    .map_err(|source| ProfileError::xml(path.to_path_buf(), source.to_string()))?;
            }
            Ok(Event::Start(event)) if is_boolean(&event) && bool_name_matches(&event, key)? => {
                writer
                    .write_event(Event::Start(rewrite_event(&event, value)?))
                    .map_err(|source| ProfileError::xml(path.to_path_buf(), source.to_string()))?;
            }
            Ok(Event::Eof) => break,
            Ok(event) => writer
                .write_event(event)
                .map_err(|source| ProfileError::xml(path.to_path_buf(), source.to_string()))?,
            Err(source) => return Err(ProfileError::xml(path.to_path_buf(), source.to_string())),
        }
        buf.clear();
    }
    Ok(writer.into_inner().into_inner())
}

fn rewrite_event(event: &BytesStart<'_>, value: bool) -> Result<BytesStart<'static>, ProfileError> {
    let name = String::from_utf8_lossy(event.name().as_ref()).into_owned();
    let mut rewritten = BytesStart::new(name);
    let value_text = if value { "true" } else { "false" };
    for attr in event.attributes() {
        let attr = attr.map_err(|source| ProfileError::xml(PathBuf::new(), source.to_string()))?;
        let key = attr.key.as_ref();
        if key == b"value" {
            rewritten.push_attribute(("value", value_text));
        } else {
            let key_owned = key.to_vec();
            let value_owned = attr.value.as_ref().to_vec();
            rewritten.push_attribute((key_owned.as_slice(), value_owned.as_slice()));
        }
    }
    Ok(rewritten)
}

fn bool_name_matches(event: &BytesStart<'_>, expected: &str) -> Result<bool, ProfileError> {
    for attr in event.attributes() {
        let attr = attr.map_err(|source| ProfileError::xml(PathBuf::new(), source.to_string()))?;
        if attr.key.as_ref() == b"name" {
            return Ok(attr.value == Cow::Borrowed(expected.as_bytes()));
        }
    }
    Ok(false)
}

fn bool_value(event: &BytesStart<'_>) -> Result<bool, ProfileError> {
    for attr in event.attributes() {
        let attr = attr.map_err(|source| ProfileError::xml(PathBuf::new(), source.to_string()))?;
        if attr.key.as_ref() == b"value" {
            return match attr.value.as_ref() {
                b"true" => Ok(true),
                b"false" => Ok(false),
                value => Err(ProfileError::xml(
                    PathBuf::new(),
                    format!(
                        "unsupported boolean value: {}",
                        String::from_utf8_lossy(value)
                    ),
                )),
            };
        }
    }
    Err(ProfileError::xml(PathBuf::new(), "missing boolean value"))
}

fn is_boolean(event: &BytesStart<'_>) -> bool {
    event.name().as_ref() == b"boolean"
}
