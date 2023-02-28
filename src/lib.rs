use pyo3::prelude::*;
use rpds::HashTrieMap;

#[derive(Hash)]
#[pyclass(mapping, name = "HashTrieMap")]
struct HashTrieMapPy {}

impl From<HashTrieMap<String, PyAny>> for HashTrieMapPy {
    fn from(_map: HashTrieMap<String, PyAny>) -> Self {
        HashTrieMapPy {}
    }
}

#[pymethods]
impl HashTrieMapPy {
    #[new]
    fn init(_value: &PyAny) -> Self {
        HashTrieMapPy {}
    }

    fn __getitem__(&self, key: &PyAny) -> PyResult<f64> {
        Ok(0.0)
    }

    fn insert(&self, key: &PyAny, value: &PyAny) -> PyResult<HashTrieMapPy> {
        Ok(HashTrieMapPy {})
    }
}

#[derive(Hash)]
#[pyclass(name = "HashTrieSet")]
struct HashTrieSetPy {}

#[pymethods]
impl HashTrieSetPy {
    #[new]
    fn init() -> Self {
        HashTrieSetPy {}
    }

    fn insert(&self, value: &PyAny) -> PyResult<HashTrieSetPy> {
        Ok(HashTrieSetPy {})
    }
}

#[derive(Hash)]
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
