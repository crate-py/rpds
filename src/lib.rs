use std::hash::{Hash, Hasher};
use std::vec::IntoIter;

use pyo3::exceptions::PyIndexError;
use pyo3::pyclass::CompareOp;
use pyo3::types::{PyDict, PyIterator, PyList, PyTuple};
use pyo3::{exceptions::PyKeyError, types::PyMapping};
use pyo3::{prelude::*, AsPyPointer};
use rpds::{HashTrieMap, HashTrieSet, List};

#[derive(Clone, Debug)]
struct Key {
    hash: isize,
    inner: PyObject,
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_isize(self.hash);
    }
}

impl Eq for Key {}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        Python::with_gil(|py| {
            self.inner
                .call_method1(py, "__eq__", (&other.inner,))
                .and_then(|value| value.extract(py))
                .expect("__eq__ failed!")
        })
    }
}

impl IntoPy<PyObject> for Key {
    fn into_py(self, py: Python<'_>) -> PyObject {
        self.inner.into_py(py)
    }
}

impl AsPyPointer for Key {
    fn as_ptr(&self) -> *mut pyo3::ffi::PyObject {
        self.inner.as_ptr()
    }
}

impl<'source> FromPyObject<'source> for Key {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        Ok(Key {
            hash: ob.hash()?,
            inner: ob.into(),
        })
    }
}

#[repr(transparent)]
#[pyclass(name = "HashTrieMap", module = "rpds", frozen, mapping, unsendable)]
struct HashTrieMapPy {
    inner: HashTrieMap<Key, PyObject>,
}

impl From<HashTrieMap<Key, PyObject>> for HashTrieMapPy {
    fn from(map: HashTrieMap<Key, PyObject>) -> Self {
        HashTrieMapPy { inner: map }
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

    fn __len__(&self) -> usize {
        self.inner.size().into()
    }

    fn __repr__(&self, py: Python) -> String {
        let contents = self.inner.into_iter().map(|(k, v)| {
            format!(
                "{}: {}",
                k.into_py(py),
                v.call_method0(py, "__repr__")
                    .and_then(|r| r.extract(py))
                    .unwrap_or("<repr error>".to_owned())
            )
        });
        format!(
            "HashTrieMap({{{}}})",
            contents.collect::<Vec<_>>().join(", ")
        )
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.inner.size() == other.inner.size()).into_py(py),
            CompareOp::Ne => (self.inner.size() != other.inner.size()).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn get(&self, key: Key) -> Option<&PyObject> {
        self.inner.get(&key)
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

    fn insert(&self, key: Key, value: &PyAny) -> HashTrieMapPy {
        HashTrieMapPy {
            inner: self.inner.insert(Key::from(key), value.into()),
        }
    }

    fn remove(&self, key: Key) -> PyResult<HashTrieMapPy> {
        match self.inner.contains_key(&key) {
            true => Ok(HashTrieMapPy {
                inner: self.inner.remove(&key),
            }),
            false => Err(PyKeyError::new_err(key)),
        }
    }

    #[pyo3(signature = (*maps, **kwds))]
    fn update(&self, maps: &PyTuple, kwds: Option<&PyDict>) -> PyResult<HashTrieMapPy> {
        let mut inner = self.inner.clone();
        for value in maps {
            let map = HashTrieMapPy::extract(value)?;
            for (k, v) in &map.inner {
                inner.insert_mut(k.to_owned(), v.to_owned());
            }
        }
        if let Some(kwds) = kwds {
            for (k, v) in kwds {
                inner.insert_mut(Key::extract(k)?, v.extract()?);
            }
        }
        Ok(HashTrieMapPy { inner })
    }
}

#[pyclass(module = "rpds", unsendable)]
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

#[repr(transparent)]
#[pyclass(name = "HashTrieSet", module = "rpds", frozen, unsendable)]
struct HashTrieSetPy {
    inner: HashTrieSet<Key>,
}

impl<'source> FromPyObject<'source> for HashTrieSetPy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = HashTrieSet::new();
        for each in ob.iter()? {
            let k: Key = each?.extract()?;
            ret.insert_mut(k);
        }
        Ok(HashTrieSetPy { inner: ret })
    }
}

#[pymethods]
impl HashTrieSetPy {
    #[new]
    fn init(value: Option<HashTrieSetPy>) -> Self {
        if let Some(value) = value {
            value
        } else {
            HashTrieSetPy {
                inner: HashTrieSet::new(),
            }
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

    fn __len__(&self) -> usize {
        self.inner.size().into()
    }

    fn __repr__(&self, py: Python) -> String {
        let contents = self.inner.into_iter().map(|k| {
            k.into_py(py)
                .call_method0(py, "__repr__")
                .and_then(|r| r.extract(py))
                .unwrap_or("<repr failed>".to_owned())
        });
        format!(
            "HashTrieSet({{{}}})",
            contents.collect::<Vec<_>>().join(", ")
        )
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyObject {
        match op {
            CompareOp::Eq => (self.inner.size() == other.inner.size()).into_py(py),
            CompareOp::Ne => (self.inner.size() != other.inner.size()).into_py(py),
            _ => py.NotImplemented(),
        }
    }

    fn insert(&self, value: Key) -> HashTrieSetPy {
        HashTrieSetPy {
            inner: self.inner.insert(Key::from(value)),
        }
    }

    fn discard(&self, value: Key) -> PyResult<HashTrieSetPy> {
        match self.inner.contains(&value) {
            true => Ok(HashTrieSetPy {
                inner: self.inner.remove(&value),
            }),
            false => Ok(HashTrieSetPy {
                inner: self.inner.clone(),
            }),
        }
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
#[pyclass(name = "List", module = "rpds", frozen, sequence, unsendable)]
struct ListPy {
    inner: List<PyObject>,
}

impl From<List<PyObject>> for ListPy {
    fn from(elements: List<PyObject>) -> Self {
        ListPy { inner: elements }
    }
}

impl<'source> FromPyObject<'source> for ListPy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = List::new();
        let reversed: &PyIterator = ob.call_method0("__reversed__")?.downcast()?;
        for each in reversed {
            ret.push_front_mut(each?.extract()?);
        }
        Ok(ListPy { inner: ret })
    }
}

#[pymethods]
impl ListPy {
    #[new]
    #[pyo3(signature = (*elements))]
    fn init(elements: &PyTuple) -> PyResult<Self> {
        let mut ret: ListPy;
        if elements.len() == 1 {
            ret = elements.get_item(0)?.extract()?;
        } else {
            ret = ListPy { inner: List::new() };
            if elements.len() > 1 {
                for each in (0..elements.len()).rev() {
                    ret.inner
                        .push_front_mut(elements.get_item(each)?.extract()?);
                }
            }
        }
        Ok(ret)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self, py: Python) -> String {
        let contents = self.inner.into_iter().map(|k| {
            k.into_py(py)
                .call_method0(py, "__repr__")
                .and_then(|r| r.extract(py))
                .unwrap_or("<repr failed>".to_owned())
        });
        format!("List([{}])", contents.collect::<Vec<_>>().join(", "))
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyResult<PyObject> {
        match op {
            CompareOp::Eq => Ok((self.inner.len() == other.inner.len()
                && self
                    .inner
                    .iter()
                    .zip(other.inner.iter())
                    .map(|(e1, e2)| PyAny::eq(e1.extract(py)?, e2))
                    .all(|r| r.unwrap_or(false)))
            .into_py(py)),
            _ => Ok(py.NotImplemented()),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<ListIterator>> {
        let iter = slf
            .inner
            .iter()
            .map(|k| k.to_owned())
            .collect::<Vec<_>>()
            .into_iter();
        Py::new(slf.py(), ListIterator { inner: iter })
    }

    fn __reversed__(&self) -> ListPy {
        ListPy {
            inner: self.inner.reverse(),
        }
    }

    #[getter]
    fn first(&self) -> PyResult<&PyObject> {
        self.inner
            .first()
            .ok_or_else(|| PyIndexError::new_err("empty list has no first element"))
    }

    fn push_front(&self, other: PyObject) -> ListPy {
        ListPy {
            inner: self.inner.push_front(other),
        }
    }

    #[getter]
    fn rest(&self) -> ListPy {
        let mut inner = self.inner.clone();
        inner.drop_first_mut();
        ListPy { inner }
    }
}

#[pyclass(module = "rpds", unsendable)]
struct ListIterator {
    inner: IntoIter<PyObject>,
}

#[pymethods]
impl ListIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyObject> {
        slf.inner.next()
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
