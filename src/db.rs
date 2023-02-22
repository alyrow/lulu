use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf, StripPrefixError};

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
    if !is_comparable(v1) || !is_comparable(v2) {
        Ordering::Equal
    } else {
        match v1 {
            Value::Bool(a) => match v2 {
                Value::Bool(b) => a.cmp(b),
                _ => Ordering::Equal,
            },
            Value::Number(a) => match v2 {
                Value::Number(b) => a.as_f64().map_or(Ordering::Equal, |a| {
                    b.as_f64().map_or(Ordering::Equal, |b| a.total_cmp(&b))
                }),
                _ => Ordering::Equal,
            },
            Value::String(a) => match v2 {
                Value::String(b) => a.cmp(b),
                _ => Ordering::Equal,
            },
            _ => Ordering::Equal,
        }
    }
}

fn value_cond(v1: Value, cond: Condition, v2: Value) -> bool {
    match cond {
        Condition::Equal => v1 == v2,
        Condition::NotEqual => v1 != v2,
        _ => match v1 {
            Value::Number(_) => match cond {
                Condition::Greater => v1.as_f64().unwrap() > v2.as_f64().unwrap(),
                Condition::Less => v1.as_f64().unwrap() < v2.as_f64().unwrap(),
                Condition::GreaterOrEqual => v1.as_f64().unwrap() >= v2.as_f64().unwrap(),
                Condition::LessOrEqual => v1.as_f64().unwrap() <= v2.as_f64().unwrap(),
                _ => false,
            },
            Value::String(_) => match cond {
                Condition::Greater => v1.as_str().unwrap() > v2.as_str().unwrap(),
                Condition::Less => v1.as_str().unwrap() < v2.as_str().unwrap(),
                Condition::GreaterOrEqual => v1.as_str().unwrap() >= v2.as_str().unwrap(),
                Condition::LessOrEqual => v1.as_str().unwrap() <= v2.as_str().unwrap(),
                _ => false,
            },
            Value::Bool(_) => match cond {
                Condition::Greater => v1.as_bool().unwrap() > v2.as_bool().unwrap(),
                Condition::Less => v1.as_bool().unwrap() < v2.as_bool().unwrap(),
                Condition::GreaterOrEqual => v1.as_bool().unwrap() >= v2.as_bool().unwrap(),
                Condition::LessOrEqual => v1.as_bool().unwrap() <= v2.as_bool().unwrap(),
                _ => false,
            },
            _ => false,
        },
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

    pub fn wherr(self, key: String, cond: Condition, value: Value) -> Result<Where, Error> {
        Where::new(self, key, cond, value)
    }

    pub fn get(&self) -> Vec<IdDocument> {
        if !self.exist {
            vec![]
        } else {
            self.path
                .read_dir()
                .unwrap()
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
                })
                .collect()
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
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }?;
        let file = std::fs::File::open(self.db.base.join("index.json"))?;
        let mut buf_reader = std::io::BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        let mut index: Map<String, Value> = serde_json::from_str(contents.as_str())?;
        let mut map = index
            .get(relative_path.display().to_string().as_str())
            .map_or(Map::<String, Value>::new(), |v| {
                v.as_object()
                    .unwrap_or(&Map::<String, Value>::new())
                    .to_owned()
            });
        // TODO No index
        let docs = self.get();
        let data: Map<String, Value> = serde_json::from_str(&serde_json::to_string(&data)?)?;
        data.keys().for_each(|key| {
            let mut docs: Vec<(String, Map<String, Value>)> = docs
                .iter()
                .filter(|doc| match doc.doc.clone().get::<Map<String, Value>>() {
                    Ok(opt) => match opt {
                        None => false,
                        Some(d) => d.contains_key(key) && is_comparable(d.get(key).unwrap()),
                    },
                    Err(_) => false,
                })
                .map(|d| {
                    (
                        d.id.clone(),
                        d.doc.clone().get::<Map<String, Value>>().unwrap().unwrap(),
                    )
                })
                .collect();
            if docs.len() != 0 {
                docs.sort_by(|a, b| value_cmp(a.1.get(key).unwrap(), b.1.get(key).unwrap()));
                let a: Vec<Vec<Value>> = docs
                    .iter()
                    .map(|doc| {
                        let mut v = Vec::<Value>::new();
                        v.insert(0, doc.1.get(key).unwrap().clone().take());
                        v.insert(1, Value::from(doc.0.clone()));
                        v
                    })
                    .collect();
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

#[derive(Clone)]
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
        self.collection.mkdir()?;
        let serialized = serde_json::to_string(&data).unwrap();
        let mut file = std::fs::File::create(self.path.clone())?;
        file.write_all(serialized.as_ref())?;
        if index {
            self.collection.index(data)?;
        }
        Ok(())
    }

    pub fn get<T>(self) -> Result<Option<T>, Error>
    where
        T: DeserializeOwned,
    {
        if self.exist {
            let file = std::fs::File::open(self.path.clone())?;
            let mut buf_reader = std::io::BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents)?;

            let deserialized: T = serde_json::from_str::<T>(contents.as_str())?;
            Ok(Some(deserialized))
        } else {
            Ok(None)
        }
    }

    pub fn update<T: Serialize>(&mut self, data: T) -> Result<(), Error>
    where
        T: DeserializeOwned,
    {
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

#[derive(Clone)]
pub struct Where {
    collection: Collection,
    result: Vec<IdDocument>,
}

#[derive(Clone)]
pub enum Condition {
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterOrEqual,
    LessOrEqual,
}

impl Where {
    pub fn new(
        collection: Collection,
        key: String,
        cond: Condition,
        value: Value,
    ) -> Result<Where, Error> {
        let w = Where {
            collection: collection.copy(),
            result: Vec::new(),
        };
        let result = w.search(key, cond, value)?;
        Ok(Where { collection, result })
    }

    fn search(self, key: String, cond: Condition, value: Value) -> Result<Vec<IdDocument>, Error> {
        if !self.collection.exist {
            return Ok(vec![]);
        }
        let relative_path = match self
            .collection
            .path
            .strip_prefix(self.collection.db.base.clone())
        {
            Ok(p) => Ok(p),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }?;
        let file = std::fs::File::open(self.collection.db.base.join("index.json"))?;
        let mut buf_reader = std::io::BufReader::new(file);
        let mut contents = String::new();
        buf_reader.read_to_string(&mut contents)?;
        let index = serde_json::from_str::<Map<String, Value>>(contents.as_str())?
            .get(relative_path.display().to_string().as_str())
            .map_or(Map::<String, Value>::new(), |v| {
                v.as_object()
                    .unwrap_or(&Map::<String, Value>::new())
                    .to_owned()
            });

        if !index.contains_key(key.as_str()) {
            return Ok(vec![]);
        }

        let sorted: Vec<_> = index
            .get(key.as_str())
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    v.as_array().unwrap().get(0).unwrap(),
                    v.as_array().unwrap().get(1).unwrap().as_str().unwrap(),
                )
            })
            .collect();
        // sorted

        let mut result: Vec<(&Value, &str)>;

        match cond {
            Condition::NotEqual => {
                let r = Where::get_equal(sorted.clone(), value);
                let itv = r.0..r.0 + r.1;
                result = Vec::<(&Value, &str)>::with_capacity(sorted.len() - r.1);
                for i in 0..sorted.len() {
                    if !itv.contains(&i) {
                        result.push(sorted[i]);
                    }
                }
            }
            _ => {
                let r = match cond {
                    Condition::Equal => Where::get_equal(sorted.clone(), value),
                    Condition::Less => Where::get_greater(sorted.clone(), value, true, None),
                    Condition::Greater => Where::get_less(sorted.clone(), value, true),
                    Condition::LessOrEqual => {
                        Where::get_greater(sorted.clone(), value, false, None)
                    }
                    Condition::GreaterOrEqual => Where::get_less(sorted.clone(), value, false),
                    _ => (0, 0),
                };
                result = Vec::<(&Value, &str)>::with_capacity(r.1);
                for i in 0..r.1 {
                    result.insert(i, sorted[r.0 + i]);
                }
            }
        }

        Ok(result
            .iter()
            .map(|e| IdDocument::new(e.1.to_string(), Document::new(e.1, self.collection.copy())))
            .collect())
    }

    pub fn wherr(&mut self, key: String, cond: Condition, value: Value) -> Result<Where, Error> {
        let v = self
            .clone()
            .search(key, cond, value)?
            .iter()
            .map(|e| e.id.clone())
            .collect::<HashSet<String>>();
        self.result = self
            .result
            .iter()
            .map(|e| e.id.clone())
            .collect::<HashSet<String>>()
            .intersection(&v)
            .map(|e| {
                IdDocument::new(
                    e.to_owned(),
                    Document::new(e.as_str(), self.collection.copy()),
                )
            })
            .collect();

        Ok(self.clone())
    }

    pub fn get(self) -> Vec<IdDocument> {
        self.result
    }

    fn get_greater(
        v: Vec<(&Value, &str)>,
        val: Value,
        strict: bool,
        bounds: Option<(usize, usize)>,
    ) -> (usize, usize) {
        let mut bounds = bounds.unwrap_or((0, v.len() - 1));
        if bounds.0 >= bounds.1 {
            if value_cmp(v[bounds.1].0, &val) == Ordering::Less
                || (!strict && value_cmp(v[bounds.1].0, &val) == Ordering::Equal)
            {
                (0, bounds.1 + 1)
            } else {
                (0, 0)
            }
        } else {
            if value_cmp(v[(bounds.0 + bounds.1 + 1) / 2].0, &val) == Ordering::Less
                || (!strict
                    && value_cmp(v[(bounds.0 + bounds.1 + 1) / 2].0, &val) == Ordering::Equal)
            {
                bounds.0 = (bounds.0 + bounds.1 + 1) / 2;
                Where::get_greater(v, val, strict, Some(bounds))
            } else {
                bounds.1 = (bounds.0 + bounds.1) / 2;
                Where::get_greater(v, val, strict, Some(bounds))
            }
        }
    }

    fn get_less(v: Vec<(&Value, &str)>, val: Value, strict: bool) -> (usize, usize) {
        let g = Where::get_greater(v.clone(), val, !strict, None).1;
        (g, v.len() - g)
    }

    fn get_equal(v: Vec<(&Value, &str)>, val: Value) -> (usize, usize) {
        let less = Where::get_less(v.clone(), val.clone(), false);
        let greater = Where::get_greater(v.clone(), val, false, None);

        (less.0, greater.1 - less.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_greater() {
        let mut v = Vec::new();
        let u = (&Value::from(1), "a");
        v.push(u);
        let u = (&Value::from(2), "b");
        v.push(u);
        let u = (&Value::from(3), "c");
        v.push(u);
        let u = (&Value::from(3), "d");
        v.push(u);
        let u = (&Value::from(3), "e");
        v.push(u);
        let u = (&Value::from(4), "f");
        v.push(u);
        let u = (&Value::from(7), "g");
        v.push(u);
        let u = (&Value::from(8), "h");
        v.push(u);
        let u = (&Value::from(9), "i");
        v.push(u);
        let u = (&Value::from(9), "j");
        v.push(u);

        let bounds = Where::get_greater(v.clone(), Value::from(3), false, None);
        println!("{:?}", bounds);

        let bounds = Where::get_less(v.clone(), Value::from(3), false);
        println!("{:?}", bounds);

        let bounds = Where::get_equal(v, Value::from(3));
        println!("{:?}", bounds);
    }
}
