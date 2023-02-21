use std::cmp::Ordering;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf, StripPrefixError};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

/// Base struct of the database
///
/// This is a nosql database working with a model of collection and documents.
/// A collection contains a set of documents and a document contains data.
/// A document can contain a collection.
#[derive(Clone)]
pub struct Db {
    base: PathBuf,
}

impl Db {
    pub fn new(base: PathBuf) -> Result<Db, Error> {
        if !base.is_dir() {
            std::fs::create_dir_all(base.clone())?;
            let mut file = std::fs::File::create(base.join("index.json"))?;
            file.write_all(b"{}")?;
            let mut file = std::fs::File::create(base.join("version"))?;
            file.write_all(b"1")?;
        }
        Ok(Db { base })
    }

    pub fn collection(self, name: &str) -> Collection {
        Collection::new(self, name)
    }
}

#[derive(Clone)]
pub struct Collection {
    path: PathBuf,
    pub name: String,
    db: Db,
    pub exist: bool,
}

fn is_comparable(v: &Value) -> bool {
    v.is_number() || v.is_string() || v.is_boolean()
}

fn value_cmp(v1: &Value, v2: &Value) -> Ordering {
    if !is_comparable(v1) || !is_comparable(v2) { Ordering::Equal } else {
        match v1 {
            Value::Bool(a) => match v2 {
                Value::Bool(b) => a.cmp(b),
                _ => Ordering::Equal
            }
            Value::Number(a) => match v2 {
                Value::Number(b) => a.as_f64().map_or(Ordering::Equal, |a| b.as_f64().map_or(Ordering::Equal, |b| a.total_cmp(&b))),
                _ => Ordering::Equal
            }
            Value::String(a) => match v2 {
                Value::String(b) => a.cmp(b),
                _ => Ordering::Equal
            }
            _ => Ordering::Equal
        }
    }
}

impl Collection {
    pub fn new(db: Db, name: &str) -> Collection {
        let path = db.base.join(name);
        Collection {
            path: path.clone(),
            name: name.to_string(),
            db,
            exist: path.is_dir(),
        }
    }

    pub fn new_from(db: Db, name: &str, base: PathBuf) -> Collection {
        let path = base.join(name);
        Collection {
            path: path.clone(),
            name: name.to_string(),
            db,
            exist: path.is_dir(),
        }
    }

    pub fn doc(self, name: &str) -> Document {
        Document::new(name, self)
    }

    pub fn add<T: Serialize>(&self, data: T) -> Result<String, Error> {
        self.mkdir()?;
        let id = uuid::Uuid::new_v4().to_string();
        self.copy().doc(id.as_str()).set(data)?;
        Ok(id)
    }

    // TODO pub fn where

    pub fn get(&self) -> Vec<IdDocument> {
        if !self.exist {
            vec![]
        } else {
            self.path
                .read_dir().unwrap()
                .filter(|f| f.as_ref().unwrap().path().extension().unwrap() == "json")
                .map(|f| {
                    let name = f
                        .unwrap()
                        .file_name()
                        .into_string()
                        .unwrap()
                        .replace(".json", "");
                    let c = self.copy();
                    IdDocument::new(name.clone(), Document::new(name.as_str(), c))
                }).collect()
        }
    }

    fn mkdir(&self) -> Result<(), Error> {
        if !self.exist {
            std::fs::create_dir_all(self.path.clone())?;
        }
        Ok(())
    }

    pub fn index<T: Serialize>(&self, data: T) -> Result<(), Error> {
        let relative_path = match self.path.strip_prefix(self.db.base.clone()) {
            Ok(p) => Ok(p),
            Err(e) => Err(Error::new(ErrorKind::Other, e))
        }?;
        let file = std::fs::File::open(self.db.base.join("index.json"))?;
        let mut buf_reader = std::io::BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        let mut index: Map<String, Value> = serde_json::from_str(contents.as_str())?;
        let mut map = index.get(relative_path.display().to_string().as_str()).map_or(Map::<String, Value>::new(), |v| v.as_object().unwrap_or(&Map::<String, Value>::new()).to_owned());
        // TODO No index
        let docs = self.get();
        let data: Map<String, Value> = serde_json::from_str(&serde_json::to_string(&data)?)?;
        data.keys().for_each(|key| {
            let mut docs: Vec<(String, Map<String, Value>)> = docs.iter().filter(|doc| {
                match doc.doc.clone().get::<Map<String, Value>>() {
                    Ok(opt) => match opt {
                        None => false,
                        Some(d) => d.contains_key(key) && is_comparable(d.get(key).unwrap())
                    },
                    Err(_) => false
                }
            }).map(|d| (d.id.clone(), d.doc.clone().get::<Map<String, Value>>().unwrap().unwrap())).collect();
            if docs.len() != 0 {
                docs.sort_by(|a, b| value_cmp(a.1.get(key).unwrap(), b.1.get(key).unwrap()));
                let a: Vec<Vec<Value>> = docs.iter().map(|doc| {
                    let mut v = Vec::<Value>::new();
                    v.insert(0, doc.1.get(key).unwrap().clone().take());
                    v.insert(1, Value::from(doc.0.clone()));
                    v
                }).collect();
                map.insert(key.to_string(), Value::from(a));
            }
        });

        index.insert(relative_path.display().to_string(), Value::from(map));
        let serialized = serde_json::to_string(&index).unwrap();
        let mut file = std::fs::File::create(self.db.base.join("index.json"))?;
        file.write_all(serialized.as_ref())?;

        Ok(())
    }

    pub fn copy(&self) -> Collection {
        Collection {
            path: self.path.clone(),
            name: self.name.clone(),
            db: self.db.clone(),
            exist: self.exist,
        }
    }
}

pub struct IdDocument {
    pub id: String,
    pub doc: Document,
}

impl IdDocument {
    pub fn new(id: String, doc: Document) -> IdDocument {
        IdDocument { id, doc }
    }
}


#[derive(Clone)]
pub struct Document {
    path: PathBuf,
    pub name: String,
    collection: Collection,
    pub exist: bool,
}

impl Document {
    pub fn new(name: &str, collection: Collection) -> Document {
        let path = collection.path.join(name).with_extension("json");
        Document {
            path: path.clone(),
            name: name.to_string(),
            collection,
            exist: path.is_file(),
        }
    }

    pub fn set<T: Serialize>(&mut self, data: T) -> Result<(), Error> {
        self.set_with_index(data, true)?;
        self.exist = true;
        Ok(())
    }

    pub fn set_with_index<T: Serialize>(&self, data: T, index: bool) -> Result<(), Error> {
        let serialized = serde_json::to_string(&data).unwrap();
        let mut file = std::fs::File::create(self.path.clone())?;
        file.write_all(serialized.as_ref())?;
        if index {
            self.collection.index(data)?;
        }
        Ok(())
    }

    pub fn get<T>(self) -> Result<Option<T>, Error> where T: DeserializeOwned {
        if self.exist {
            let file = std::fs::File::open(self.path.clone())?;
            let mut buf_reader = std::io::BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents)?;

            let deserialized: T = serde_json::from_str::<T>(contents.as_str())?;
            Ok(Some(deserialized))
        } else { Ok(None) }
    }

    pub fn update<T: Serialize>(&mut self, data: T) -> Result<(), Error> where T: DeserializeOwned {
        if self.exist {
            let mut content = self.clone().get::<Map<String, Value>>()?.unwrap();
            let data: Map<String, Value> = serde_json::from_str(&serde_json::to_string(&data)?)?;
            data.keys().for_each(|k: &String| {
                content.insert(k.as_str().to_string(), data.get(k).unwrap().clone());
            });

            self.set_with_index(data.clone(), false)?;
            self.collection.index(data)?;
        }
        Ok(())
    }

    pub fn delete(&mut self) -> Result<(), Error> {
        let data = self.clone().get::<Map<String, Value>>()?;
        std::fs::remove_file(self.path.clone())?;
        self.exist = false;
        if data.is_some() {
            self.collection.index(data.unwrap())?;
        }
        Ok(())
    }

    pub fn collection(self, name: &str) -> Collection {
        let mut path = self.path.clone();
        path.set_extension("");
        Collection::new_from(self.collection.db, name, path)
    }
}
