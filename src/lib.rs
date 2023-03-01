use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use rpds::{HashTrieMap, HashTrieSet};

#[repr(transparent)]
#[pyclass(name = "HashTrieMap", mapping, unsendable)]
struct HashTrieMapPy {
    inner: HashTrieMap<String, PyObject>,
}

impl From<HashTrieMap<String, PyObject>> for HashTrieMapPy {
    fn from(map: HashTrieMap<String, PyObject>) -> Self {
        HashTrieMapPy { inner: map }
    }
}

#[pymethods]
impl HashTrieMapPy {
    #[new]
    fn init(_value: &PyAny) -> Self {
        HashTrieMapPy {
            inner: HashTrieMap::new(),
        }
    }

    fn __getitem__(&self, key: String) -> PyResult<PyObject> {
        match self.inner.get(&key.to_string()) {
            Some(value) => Ok(value.to_owned()),
            None => Err(PyKeyError::new_err(key.to_string())),
        }
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.inner.size().into())
    }

    fn __repr__(&self) -> String {
        let contents = self
            .inner
            .into_iter()
            .map(|(key, value)| key.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        format!("HashTrieMap({{{}}})", contents)
    }

    fn insert(&self, key: String, value: &PyAny) -> PyResult<HashTrieMapPy> {
        Ok(HashTrieMapPy {
            inner: self.inner.insert(key.to_string(), value.into()),
        })
    }
}

#[repr(transparent)]
#[pyclass(name = "HashTrieSet", unsendable)]
struct HashTrieSetPy {
    inner: HashTrieSet<String>,
}

#[pymethods]
impl HashTrieSetPy {
    #[new]
    fn init() -> Self {
        HashTrieSetPy {
            inner: HashTrieSet::new(),
        }
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(self.inner.size().into())
    }

    fn insert(&self, value: String) -> PyResult<HashTrieSetPy> {
        Ok(HashTrieSetPy {
            inner: self.inner.insert(value.to_string()),
        })
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
fn rpds_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<HashTrieMapPy>()?;
    m.add_class::<HashTrieSetPy>()?;
    m.add_class::<ListPy>()?;
    Ok(())
}
