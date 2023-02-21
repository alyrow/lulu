use std::any::Any;
use std::io::{Error, Read, Write};
use std::ops::Deref;
use std::path::PathBuf;
use clap::builder::Str;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

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

    pub fn doc<T>(self, name: &str) -> Document {
        Document::new(name, self)
    }

    pub fn add<T: Serialize>(&self, data: T) -> Result<String, Error> {
        self.mkdir()?;
        let id = uuid::Uuid::new_v4().to_string();
        self.copy().doc::<T>(id.as_str()).set(data)?;
        Ok(id)
    }

    // TODO pub fn where

    pub fn get<T>(&self) -> Vec<IdDocument> {
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
            // TODO Index
        }
        Ok(())
    }

    pub fn index(&self) {
        todo!()
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

    pub fn set_with_index<T: Serialize>(&self, data: T, _index: bool) -> Result<(), Error> {
        let serialized = serde_json::to_string(&data).unwrap();
        let mut file = std::fs::File::create(self.path.clone())?;
        file.write_all(serialized.as_ref())?;
        Ok(())
        // TODO Handle index
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
            let mut data: Map<String, Value> = serde_json::from_str(&serde_json::to_string(&data)?)?;
            let r = Map::new();
            data.keys().for_each(|k: &String| {
                content.insert(k.as_str().to_string(), data.get(k).unwrap().clone());
            });

            self.set(data)?;
        }
        Ok(())
    }

    pub fn delete(&mut self) -> Result<(), Error> {
        // TODO data with index?
        std::fs::remove_file(self.path.clone())?;
        self.exist = false;
        Ok(())
    }

    pub fn collection(self, name: &str) -> Collection {
        let mut path = self.path.clone();
        path.set_extension("");
        Collection::new_from(self.collection.db, name, path)
    }
}
