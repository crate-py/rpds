use std::vec::IntoIter;

use pyo3::prelude::*;
use pyo3::pyclass::CompareOp;
use pyo3::types::{PyDict, PyList};
use pyo3::{exceptions::PyKeyError, types::PyMapping};
use rpds::{HashTrieMap, HashTrieSet};

type Key = String;

#[repr(transparent)]
#[pyclass(name = "HashTrieMap", mapping, unsendable)]
struct HashTrieMapPy {
    inner: HashTrieMap<Key, PyObject>,
}

impl From<HashTrieMap<Key, PyObject>> for HashTrieMapPy {
    fn from(map: HashTrieMap<Key, PyObject>) -> Self {
        HashTrieMapPy { inner: map }
    }
}

#[pyclass(unsendable)]
struct KeyIterator {
    inner: IntoIter<Key>,
}

#[pymethods]
impl KeyIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Key> {
        slf.inner.next()
    }
}

impl<'source> FromPyObject<'source> for HashTrieMapPy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = HashTrieMap::new();
        if ob.is_instance_of::<PyList>()? {
            let sequence: &PyList = ob.downcast()?;
            for each in sequence.iter() {
                let (k, v): (Key, PyObject) = each.extract()?;
                ret.insert_mut(k, v);
            }
        } else {
            let dict: &PyDict = ob.downcast()?;
            for (k, v) in dict {
                ret.insert_mut(Key::extract(k)?, v.extract()?);
            }
        }
        Ok(HashTrieMapPy { inner: ret })
    }
}

#[pymethods]
impl HashTrieMapPy {
    #[new]
    #[pyo3(signature = (value=None, **kwds))]
    fn init(value: Option<HashTrieMapPy>, kwds: Option<&PyDict>) -> PyResult<Self> {
        let mut map: HashTrieMapPy;
        if let Some(value) = value {
            map = value;
        } else {
            map = HashTrieMapPy {
                inner: HashTrieMap::new(),
            };
        }
        if let Some(kwds) = kwds {
            for (k, v) in kwds {
                map.inner.insert_mut(Key::extract(k)?, v.into());
            }
        }
        Ok(map)
    }

    fn __contains__(&self, key: Key) -> bool {
        self.inner.contains_key(&key)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<KeyIterator>> {
        Py::new(
            slf.py(),
            KeyIterator {
                inner: slf.keys().into_iter(),
            },
        )
    }

    fn __getitem__(&self, key: Key) -> PyResult<PyObject> {
        match self.inner.get(&key) {
            Some(value) => Ok(value.to_owned()),
            None => Err(PyKeyError::new_err(key)),
        }
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.inner.size().into())
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|(k, v)| {
            format!(
                "{}: {}",
                k.into_py(py),
                v.call_method0(py, "__repr__")
                    .and_then(|r| r.extract(py))
                    .unwrap_or("<repr error>".to_owned())
            )
        });
        Ok(format!(
            "HashTrieMap({{{}}})",
            contents.collect::<Vec<_>>().join(", ")
        ))
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.inner.size() == other.inner.size()).into_py(py),
            CompareOp::Ne => (self.inner.size() != other.inner.size()).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn keys(&self) -> Vec<Key> {
        self.inner.keys().map(|key| key.clone()).collect()
    }

    fn values(&self) -> Vec<&PyObject> {
        self.inner.values().collect::<Vec<&PyObject>>().to_owned()
    }

    fn items(&self) -> Vec<(&Key, &PyObject)> {
        self.inner
            .iter()
            .collect::<Vec<(&Key, &PyObject)>>()
            .to_owned()
    }

    fn remove(&self, key: Key) -> PyResult<HashTrieMapPy> {
        match self.inner.contains_key(&key) {
            true => Ok(HashTrieMapPy {
                inner: self.inner.remove(&key),
            }),
            false => Err(PyKeyError::new_err(key)),
        }
    }

    fn insert(&self, key: Key, value: &PyAny) -> PyResult<HashTrieMapPy> {
        Ok(HashTrieMapPy {
            inner: self.inner.insert(Key::from(key), value.into()),
        })
    }
}

#[repr(transparent)]
#[pyclass(name = "HashTrieSet", unsendable)]
struct HashTrieSetPy {
    inner: HashTrieSet<Key>,
}

impl<'source> FromPyObject<'source> for HashTrieSetPy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = HashTrieSet::new();
        let sequence: &PyList = ob.downcast()?;
        for each in sequence.iter() {
            let k: Key = each.extract()?;
            ret.insert_mut(k);
        }
        Ok(HashTrieSetPy { inner: ret })
    }
}

#[pymethods]
impl HashTrieSetPy {
    #[new]
    fn init(value: Option<HashTrieSetPy>) -> PyResult<Self> {
        if let Some(value) = value {
            Ok(value)
        } else {
            Ok(HashTrieSetPy {
                inner: HashTrieSet::new(),
            })
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<KeyIterator>> {
        let iter = slf
            .inner
            .iter()
            .map(|k| k.to_owned())
            .collect::<Vec<_>>()
            .into_iter();
        Py::new(slf.py(), KeyIterator { inner: iter })
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.inner.size().into())
    }

    fn __repr__(&self) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|k| format!("{:?}", k));
        Ok(format!(
            "HashTrieSet([{}])",
            contents.collect::<Vec<_>>().join(", ")
        ))
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.inner.size() == other.inner.size()).into_py(py),
            CompareOp::Ne => (self.inner.size() != other.inner.size()).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn insert(&self, value: Key) -> PyResult<HashTrieSetPy> {
        Ok(HashTrieSetPy {
            inner: self.inner.insert(Key::from(value)),
        })
    }

    fn remove(&self, value: Key) -> PyResult<HashTrieSetPy> {
        match self.inner.contains(&value) {
            true => Ok(HashTrieSetPy {
                inner: self.inner.remove(&value),
            }),
            false => Err(PyKeyError::new_err(value)),
        }
    }
}

#[repr(transparent)]
#[pyclass(name = "List")]
struct ListPy {}

#[pymethods]
impl ListPy {
    #[new]
    fn init() -> Self {
        ListPy {}
    }
}

#[pymodule]
#[pyo3(name = "rpds")]
fn rpds_py(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<HashTrieMapPy>()?;
    PyMapping::register::<HashTrieMapPy>(py)?;
    m.add_class::<HashTrieSetPy>()?;
    m.add_class::<ListPy>()?;
    Ok(())
}
