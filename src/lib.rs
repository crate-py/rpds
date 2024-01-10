use std::hash::{Hash, Hasher};

use pyo3::exceptions::PyIndexError;
use pyo3::pyclass::CompareOp;
use pyo3::types::{PyDict, PyIterator, PyTuple, PyType};
use pyo3::{exceptions::PyKeyError, types::PyMapping};
use pyo3::{prelude::*, AsPyPointer, PyTypeInfo};
use rpds::{
    HashTrieMap, HashTrieMapSync, HashTrieSet, HashTrieSetSync, List, ListSync, Queue, QueueSync,
};

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

unsafe impl AsPyPointer for Key {
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
#[pyclass(name = "HashTrieMap", module = "rpds", frozen, mapping)]
struct HashTrieMapPy {
    inner: HashTrieMapSync<Key, PyObject>,
}

impl From<HashTrieMapSync<Key, PyObject>> for HashTrieMapPy {
    fn from(map: HashTrieMapSync<Key, PyObject>) -> Self {
        HashTrieMapPy { inner: map }
    }
}

impl<'source> FromPyObject<'source> for HashTrieMapPy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = HashTrieMap::new_sync();
        if let Ok(mapping) = ob.downcast::<PyMapping>() {
            for each in mapping.items()?.iter()? {
                let (k, v): (Key, PyObject) = each?.extract()?;
                ret.insert_mut(k, v);
            }
        } else {
            for each in ob.iter()? {
                let (k, v): (Key, PyObject) = each?.extract()?;
                ret.insert_mut(k, v);
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
                inner: HashTrieMap::new_sync(),
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

    fn __iter__(slf: PyRef<'_, Self>) -> KeyIterator {
        KeyIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __getitem__(&self, key: Key) -> PyResult<PyObject> {
        match self.inner.get(&key) {
            Some(value) => Ok(value.to_owned()),
            None => Err(PyKeyError::new_err(key)),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.size()
    }

    fn __repr__(&self, py: Python) -> String {
        let contents = self.inner.into_iter().map(|(k, v)| {
            format!(
                "{}: {}",
                k.inner
                    .call_method0(py, "__repr__")
                    .and_then(|r| r.extract(py))
                    .unwrap_or("<repr error>".to_owned()),
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

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyResult<PyObject> {
        match op {
            CompareOp::Eq => Ok((self.inner.size() == other.inner.size()
                && self
                    .inner
                    .iter()
                    .map(|(k1, v1)| (v1, other.inner.get(k1)))
                    .map(|(v1, v2)| PyAny::eq(v1.extract(py)?, v2))
                    .all(|r| r.unwrap_or(false)))
            .into_py(py)),
            CompareOp::Ne => Ok((self.inner.size() != other.inner.size()
                || self
                    .inner
                    .iter()
                    .map(|(k1, v1)| (v1, other.inner.get(k1)))
                    .map(|(v1, v2)| PyAny::ne(v1.extract(py)?, v2))
                    .all(|r| r.unwrap_or(true)))
            .into_py(py)),
            _ => Ok(py.NotImplemented()),
        }
    }

    fn __reduce__(slf: PyRef<Self>) -> (&PyType, (Vec<(Key, PyObject)>,)) {
        (
            HashTrieMapPy::type_object(slf.py()),
            (slf.inner
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),),
        )
    }

    #[classmethod]
    fn convert(_cls: &PyType, value: &PyAny, py: Python) -> PyResult<PyObject> {
        if value.is_instance_of::<HashTrieMapPy>() {
            Ok(value.into())
        } else {
            Ok(HashTrieMapPy::extract(value)?.into_py(py))
        }
    }

    #[classmethod]
    fn fromkeys(
        _cls: &PyType,
        keys: &PyAny,
        val: Option<&PyAny>,
        py: Python,
    ) -> PyResult<HashTrieMapPy> {
        let mut inner = HashTrieMap::new_sync();
        let none = py.None();
        let value = val.unwrap_or_else(|| none.as_ref(py));
        for each in keys.iter()? {
            let key = Key::extract(each?)?.to_owned();
            inner.insert_mut(key, value.into());
        }
        Ok(HashTrieMapPy { inner })
    }

    fn get(&self, key: Key, default: Option<PyObject>) -> Option<PyObject> {
        if let Some(value) = self.inner.get(&key) {
            Some(value.to_owned())
        } else {
            default
        }
    }

    fn keys(&self) -> Vec<Key> {
        self.inner.keys().cloned().collect()
    }

    fn values(&self) -> Vec<&PyObject> {
        self.inner.values().collect::<Vec<&PyObject>>()
    }

    fn items(&self) -> Vec<(Key, &PyObject)> {
        self.inner.iter().map(|(k, v)| (k.clone(), v)).collect()
    }

    fn discard(&self, key: Key) -> PyResult<HashTrieMapPy> {
        match self.inner.contains_key(&key) {
            true => Ok(HashTrieMapPy {
                inner: self.inner.remove(&key),
            }),
            false => Ok(HashTrieMapPy {
                inner: self.inner.clone(),
            }),
        }
    }

    fn insert(&self, key: Key, value: &PyAny) -> HashTrieMapPy {
        HashTrieMapPy {
            inner: self.inner.insert(key, value.into()),
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

#[pyclass(module = "rpds")]
struct KeyIterator {
    inner: HashTrieMapSync<Key, PyObject>,
}

#[pymethods]
impl KeyIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Key> {
        let first = slf.inner.keys().next()?.to_owned();
        slf.inner = slf.inner.remove(&first);
        Some(first)
    }
}

#[repr(transparent)]
#[pyclass(name = "HashTrieSet", module = "rpds", frozen)]
struct HashTrieSetPy {
    inner: HashTrieSetSync<Key>,
}

impl<'source> FromPyObject<'source> for HashTrieSetPy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = HashTrieSet::new_sync();
        for each in ob.iter()? {
            let k: Key = each?.extract()?;
            ret.insert_mut(k);
        }
        Ok(HashTrieSetPy { inner: ret })
    }
}

fn is_subset(one: &HashTrieSetSync<Key>, two: &HashTrieSetSync<Key>) -> bool {
    one.iter().all(|v| two.contains(v))
}

#[pymethods]
impl HashTrieSetPy {
    #[new]
    fn init(value: Option<HashTrieSetPy>) -> Self {
        if let Some(value) = value {
            value
        } else {
            HashTrieSetPy {
                inner: HashTrieSet::new_sync(),
            }
        }
    }

    fn __and__(&self, other: &Self) -> Self {
        self.intersection(other)
    }

    fn __or__(&self, other: &Self) -> Self {
        self.union(other)
    }

    fn __sub__(&self, other: &Self) -> Self {
        self.difference(other)
    }

    fn __xor__(&self, other: &Self) -> Self {
        self.symmetric_difference(other)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> SetIterator {
        SetIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.size()
    }

    fn __repr__(&self, py: Python) -> String {
        let contents = self.inner.into_iter().map(|k| {
            k.clone()
                .into_py(py)
                .call_method0(py, "__repr__")
                .and_then(|r| r.extract(py))
                .unwrap_or("<repr failed>".to_owned())
        });
        format!(
            "HashTrieSet({{{}}})",
            contents.collect::<Vec<_>>().join(", ")
        )
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyResult<PyObject> {
        match op {
            CompareOp::Eq => Ok((self.inner.size() == other.inner.size()
                && is_subset(&self.inner, &other.inner))
            .into_py(py)),
            CompareOp::Ne => Ok((self.inner.size() != other.inner.size()
                || self.inner.iter().any(|k| !other.inner.contains(k)))
            .into_py(py)),
            CompareOp::Lt => Ok((self.inner.size() < other.inner.size()
                && is_subset(&self.inner, &other.inner))
            .into_py(py)),
            CompareOp::Le => Ok(is_subset(&self.inner, &other.inner).into_py(py)),
            _ => Ok(py.NotImplemented()),
        }
    }

    fn __reduce__(slf: PyRef<Self>) -> (&PyType, (Vec<Key>,)) {
        (
            HashTrieSetPy::type_object(slf.py()),
            (slf.inner.iter().cloned().collect(),),
        )
    }

    fn insert(&self, value: Key) -> HashTrieSetPy {
        HashTrieSetPy {
            inner: self.inner.insert(value),
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

    fn difference(&self, other: &Self) -> HashTrieSetPy {
        let mut inner = self.inner.clone();
        for value in other.inner.iter() {
            inner.remove_mut(value);
        }
        HashTrieSetPy { inner }
    }

    fn intersection(&self, other: &Self) -> HashTrieSetPy {
        let mut inner: HashTrieSetSync<Key> = HashTrieSet::new_sync();
        let larger: &HashTrieSetSync<Key>;
        let iter;
        if self.inner.size() > other.inner.size() {
            larger = &self.inner;
            iter = other.inner.iter();
        } else {
            larger = &other.inner;
            iter = self.inner.iter();
        }
        for value in iter {
            if larger.contains(value) {
                inner.insert_mut(value.to_owned());
            }
        }
        HashTrieSetPy { inner }
    }

    fn symmetric_difference(&self, other: &Self) -> HashTrieSetPy {
        let mut inner: HashTrieSetSync<Key>;
        let iter;
        if self.inner.size() > other.inner.size() {
            inner = self.inner.clone();
            iter = other.inner.iter();
        } else {
            inner = other.inner.clone();
            iter = self.inner.iter();
        }
        for value in iter {
            if inner.contains(value) {
                inner.remove_mut(value);
            } else {
                inner.insert_mut(value.to_owned());
            }
        }
        HashTrieSetPy { inner }
    }

    fn union(&self, other: &Self) -> HashTrieSetPy {
        let mut inner: HashTrieSetSync<Key>;
        let iter;
        if self.inner.size() > other.inner.size() {
            inner = self.inner.clone();
            iter = other.inner.iter();
        } else {
            inner = other.inner.clone();
            iter = self.inner.iter();
        }
        for value in iter {
            inner.insert_mut(value.to_owned());
        }
        HashTrieSetPy { inner }
    }

    #[pyo3(signature = (*iterables))]
    fn update(&self, iterables: &PyTuple) -> PyResult<HashTrieSetPy> {
        let mut inner = self.inner.clone();
        for each in iterables {
            let iter = each.iter()?;
            for value in iter {
                inner.insert_mut(Key::extract(value?)?.to_owned());
            }
        }
        Ok(HashTrieSetPy { inner })
    }
}

#[pyclass(module = "rpds")]
struct SetIterator {
    inner: HashTrieSetSync<Key>,
}

#[pymethods]
impl SetIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Key> {
        let first = slf.inner.iter().next()?.to_owned();
        slf.inner = slf.inner.remove(&first);
        Some(first)
    }
}

#[repr(transparent)]
#[pyclass(name = "List", module = "rpds", frozen, sequence)]
struct ListPy {
    inner: ListSync<PyObject>,
}

impl From<ListSync<PyObject>> for ListPy {
    fn from(elements: ListSync<PyObject>) -> Self {
        ListPy { inner: elements }
    }
}

impl<'source> FromPyObject<'source> for ListPy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = List::new_sync();
        let reversed = PyModule::import(ob.py(), "builtins")?.getattr("reversed")?;
        let rob: &PyIterator = reversed.call1((ob,))?.iter()?;
        for each in rob {
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
            ret = ListPy {
                inner: List::new_sync(),
            };
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
            CompareOp::Ne => Ok((self.inner.len() != other.inner.len()
                || self
                    .inner
                    .iter()
                    .zip(other.inner.iter())
                    .map(|(e1, e2)| PyAny::ne(e1.extract(py)?, e2))
                    .any(|r| r.unwrap_or(true)))
            .into_py(py)),
            _ => Ok(py.NotImplemented()),
        }
    }

    fn __iter__(slf: PyRef<'_, Self>) -> ListIterator {
        ListIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __reversed__(&self) -> ListPy {
        ListPy {
            inner: self.inner.reverse(),
        }
    }

    fn __reduce__(slf: PyRef<Self>) -> (&PyType, (Vec<PyObject>,)) {
        (
            ListPy::type_object(slf.py()),
            (slf.inner.iter().cloned().collect(),),
        )
    }

    #[getter]
    fn first(&self) -> PyResult<&PyObject> {
        self.inner
            .first()
            .ok_or_else(|| PyIndexError::new_err("empty list has no first element"))
    }

    #[getter]
    fn rest(&self) -> ListPy {
        let mut inner = self.inner.clone();
        inner.drop_first_mut();
        ListPy { inner }
    }

    fn push_front(&self, other: PyObject) -> ListPy {
        ListPy {
            inner: self.inner.push_front(other),
        }
    }

    fn drop_first(&self) -> PyResult<ListPy> {
        if let Some(inner) = self.inner.drop_first() {
            Ok(ListPy { inner })
        } else {
            Err(PyIndexError::new_err("empty list has no first element"))
        }
    }
}

#[pyclass(module = "rpds")]
struct ListIterator {
    inner: ListSync<PyObject>,
}

#[pymethods]
impl ListIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyObject> {
        let first = slf.inner.first()?.to_owned();
        slf.inner = slf.inner.drop_first()?;
        Some(first)
    }
}

#[pyclass(module = "rpds")]
struct QueueIterator {
    inner: QueueSync<PyObject>,
}

#[pymethods]
impl QueueIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyObject> {
        let first = slf.inner.peek()?.to_owned();
        slf.inner = slf.inner.dequeue()?;
        Some(first)
    }
}

#[repr(transparent)]
#[pyclass(name = "Queue", module = "rpds", frozen, sequence)]
struct QueuePy {
    inner: QueueSync<PyObject>,
}

impl From<QueueSync<PyObject>> for QueuePy {
    fn from(elements: QueueSync<PyObject>) -> Self {
        QueuePy { inner: elements }
    }
}

impl<'source> FromPyObject<'source> for QueuePy {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        let mut ret = Queue::new_sync();
        for each in ob.iter()? {
            ret.enqueue_mut(each?.extract()?);
        }
        Ok(QueuePy { inner: ret })
    }
}

#[pymethods]
impl QueuePy {
    #[new]
    #[pyo3(signature = (*elements))]
    fn init(elements: &PyTuple, py: Python<'_>) -> PyResult<Self> {
        let mut ret: QueuePy;
        if elements.len() == 1 {
            ret = elements.get_item(0)?.extract()?;
        } else {
            ret = QueuePy {
                inner: Queue::new_sync(),
            };
            if elements.len() > 1 {
                for each in elements {
                    ret.inner.enqueue_mut(each.into_py(py));
                }
            }
        }
        Ok(ret)
    }

    fn __eq__(&self, other: &Self, py: Python<'_>) -> bool {
        (self.inner.len() == other.inner.len())
            && self
                .inner
                .iter()
                .zip(other.inner.iter())
                .map(|(e1, e2)| PyAny::eq(e1.extract(py)?, e2))
                .all(|r| r.unwrap_or(false))
    }

    fn __ne__(&self, other: &Self, py: Python<'_>) -> bool {
        (self.inner.len() != other.inner.len())
            || self
                .inner
                .iter()
                .zip(other.inner.iter())
                .map(|(e1, e2)| PyAny::ne(e1.extract(py)?, e2))
                .any(|r| r.unwrap_or(true))
    }

    fn __iter__(slf: PyRef<'_, Self>) -> QueueIterator {
        QueueIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self, py: Python) -> String {
        let contents = self.inner.into_iter().map(|k| {
            k.clone()
                .into_py(py)
                .call_method0(py, "__repr__")
                .and_then(|r| r.extract(py))
                .unwrap_or("<repr failed>".to_owned())
        });
        format!("Queue([{}])", contents.collect::<Vec<_>>().join(", "))
    }

    #[getter]
    fn peek(&self) -> PyResult<PyObject> {
        if let Some(peeked) = self.inner.peek() {
            Ok(peeked.to_owned())
        } else {
            Err(PyIndexError::new_err("peeked an empty queue"))
        }
    }

    #[getter]
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn enqueue(&self, value: &PyAny) -> Self {
        QueuePy {
            inner: self.inner.enqueue(value.into()),
        }
    }

    fn dequeue(&self) -> PyResult<QueuePy> {
        if let Some(inner) = self.inner.dequeue() {
            Ok(QueuePy { inner })
        } else {
            Err(PyIndexError::new_err("dequeued an empty queue"))
        }
    }
}

#[pymodule]
#[pyo3(name = "rpds")]
fn rpds_py(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<HashTrieMapPy>()?;
    PyMapping::register::<HashTrieMapPy>(py)?;
    m.add_class::<HashTrieSetPy>()?;
    m.add_class::<ListPy>()?;
    m.add_class::<QueuePy>()?;
    Ok(())
}
