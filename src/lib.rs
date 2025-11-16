use pyo3::exceptions::{PyIndexError, PyTypeError};
use pyo3::pyclass::CompareOp;
use pyo3::types::{PyDict, PyIterator, PyTuple, PyType};
use pyo3::{exceptions::PyKeyError, types::PyMapping, types::PyTupleMethods};
use pyo3::{prelude::*, BoundObject, PyTypeInfo};
use rpds::{
    HashTrieMap, HashTrieMapSync, HashTrieSet, HashTrieSetSync, List, ListSync, Queue, QueueSync,
    Stack, StackSync,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn hash_shuffle_bits(h: usize) -> usize {
    ((h ^ 89869747) ^ (h << 16)).wrapping_mul(3644798167)
}

#[derive(Debug)]
struct Key {
    hash: isize,
    inner: Py<PyAny>,
}

impl<'py> IntoPyObject<'py> for Key {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.inner.into_bound(py))
    }
}

impl<'a, 'py> IntoPyObject<'py> for &'a Key {
    type Target = PyAny;
    type Output = Borrowed<'a, 'py, Self::Target>;
    type Error = std::convert::Infallible;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(self.inner.bind_borrowed(py))
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_isize(self.hash);
    }
}

impl Eq for Key {}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        Python::attach(|py| {
            self.inner
                .call_method1(py, "__eq__", (&other.inner,))
                .and_then(|value| value.extract(py))
                .expect("__eq__ failed!")
        })
    }
}

impl Key {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        Key {
            hash: self.hash,
            inner: self.inner.clone_ref(py),
        }
    }
}

impl<'py> FromPyObject<'_, 'py> for Key {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        Ok(Key {
            hash: ob.hash()?,
            inner: ob.unbind(),
        })
    }
}

#[repr(transparent)]
#[pyclass(name = "HashTrieMap", module = "rpds", frozen, mapping)]
struct HashTrieMapPy {
    inner: HashTrieMapSync<Key, Py<PyAny>>,
}

impl From<HashTrieMapSync<Key, Py<PyAny>>> for HashTrieMapPy {
    fn from(map: HashTrieMapSync<Key, Py<PyAny>>) -> Self {
        HashTrieMapPy { inner: map }
    }
}

impl<'py> FromPyObject<'_, 'py> for HashTrieMapPy {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let mut ret = HashTrieMap::new_sync();
        if let Ok(mapping) = ob.cast::<PyMapping>() {
            for each in mapping.items()?.iter() {
                let (k, v): (Key, Py<PyAny>) = each.extract()?;
                ret.insert_mut(k, v);
            }
        } else {
            for each in ob.try_iter()? {
                let (k, v) = each?.extract()?;
                ret.insert_mut(k, v);
            }
        }
        Ok(HashTrieMapPy { inner: ret })
    }
}

type PickledTypeWithVec<'a> = (Bound<'a, PyType>, (Vec<(Key, Py<PyAny>)>,));

#[pymethods]
impl HashTrieMapPy {
    #[new]
    #[pyo3(signature = (value=None, ** kwds))]
    fn init(value: Option<HashTrieMapPy>, kwds: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
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
                map.inner
                    .insert_mut(Key::extract(k.as_borrowed())?, v.into());
            }
        }
        Ok(map)
    }

    fn __contains__(&self, key: Key) -> bool {
        self.inner.contains_key(&key)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> KeysIterator {
        KeysIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __getitem__(&self, key: Key, py: Python) -> PyResult<Py<PyAny>> {
        match self.inner.get(&key) {
            Some(value) => Ok(value.clone_ref(py)),
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

    fn __richcmp__<'py>(
        &self,
        other: &Self,
        op: CompareOp,
        py: Python<'py>,
    ) -> PyResult<Py<PyAny>> {
        match op {
            CompareOp::Eq => (self.inner.size() == other.inner.size()
                && self
                    .inner
                    .iter()
                    .map(|(k1, v1)| (v1, other.inner.get(k1)))
                    .map(|(v1, v2)| v1.bind(py).eq(v2))
                    .all(|r| r.unwrap_or(false)))
            .into_pyobject(py)
            .map_err(Into::into)
            .map(BoundObject::into_any)
            .map(BoundObject::unbind),
            CompareOp::Ne => (self.inner.size() != other.inner.size()
                || self
                    .inner
                    .iter()
                    .map(|(k1, v1)| (v1, other.inner.get(k1)))
                    .map(|(v1, v2)| v1.bind(py).ne(v2))
                    .all(|r| r.unwrap_or(true)))
            .into_pyobject(py)
            .map_err(Into::into)
            .map(BoundObject::into_any)
            .map(BoundObject::unbind),
            _ => Ok(py.NotImplemented()),
        }
    }

    fn __hash__(&self, py: Python) -> PyResult<isize> {
        // modified from https://github.com/python/cpython/blob/d69529d31ccd1510843cfac1ab53bb8cb027541f/Objects/setobject.c#L715

        let mut hash_val = self
            .inner
            .iter()
            .map(|(key, val)| {
                let mut hasher = DefaultHasher::new();
                let val_bound = val.bind(py);

                let key_hash = key.hash;
                let val_hash = val_bound.hash().map_err(|_| {
                    PyTypeError::new_err(format!(
                        "Unhashable type in HashTrieMap of key {}: {}",
                        key.inner
                            .bind(py)
                            .repr()
                            .and_then(|r| r.extract())
                            .unwrap_or("<repr> error".to_string()),
                        val_bound
                            .repr()
                            .and_then(|r| r.extract())
                            .unwrap_or("<repr> error".to_string())
                    ))
                })?;

                hasher.write_isize(key_hash);
                hasher.write_isize(val_hash);

                Ok(hasher.finish() as usize)
            })
            .try_fold(0, |acc: usize, x: PyResult<usize>| {
                PyResult::<usize>::Ok(acc ^ hash_shuffle_bits(x?))
            })?;

        // factor in the number of entries in the collection
        hash_val ^= self.inner.size().wrapping_add(1).wrapping_mul(1927868237);

        // dispense patterns in the hash value
        hash_val ^= (hash_val >> 11) ^ (hash_val >> 25);
        hash_val = hash_val.wrapping_mul(69069).wrapping_add(907133923);

        Ok(hash_val as isize)
    }

    fn __reduce__(slf: PyRef<'_, Self>) -> PickledTypeWithVec<'_> {
        (
            HashTrieMapPy::type_object(slf.py()),
            (slf.inner
                .iter()
                .map(|(k, v)| (k.clone_ref(slf.py()), v.clone_ref(slf.py())))
                .collect(),),
        )
    }

    #[classmethod]
    fn convert(
        _cls: &Bound<'_, PyType>,
        value: Bound<'_, PyAny>,
        py: Python,
    ) -> PyResult<Py<PyAny>> {
        if value.is_instance_of::<HashTrieMapPy>() {
            Ok(value.unbind())
        } else {
            HashTrieMapPy::extract(value.as_borrowed())?
                .into_pyobject(py)
                .map(BoundObject::into_any)
                .map(BoundObject::unbind)
        }
    }

    #[classmethod]
    #[pyo3(signature = (keys, val=None))]
    fn fromkeys(
        _cls: &Bound<'_, PyType>,
        keys: &Bound<'_, PyAny>,
        val: Option<&Bound<'_, PyAny>>,
        py: Python,
    ) -> PyResult<HashTrieMapPy> {
        let mut inner = HashTrieMap::new_sync();
        let none = py.None().into_bound(py);
        let value = val.unwrap_or(&none);
        for each in keys.try_iter()? {
            let key = Key::extract(each?.as_borrowed())?;
            inner.insert_mut(key, value.clone().unbind());
        }
        Ok(HashTrieMapPy { inner })
    }

    #[pyo3(signature = (key, default=None))]
    fn get(&self, key: Key, default: Option<Py<PyAny>>, py: Python) -> Option<Py<PyAny>> {
        if let Some(value) = self.inner.get(&key) {
            Some(value.clone_ref(py))
        } else {
            default
        }
    }

    fn keys(&self) -> KeysView {
        KeysView {
            inner: self.inner.clone(),
        }
    }

    fn values(&self) -> ValuesView {
        ValuesView {
            inner: self.inner.clone(),
        }
    }

    fn items(&self) -> ItemsView {
        ItemsView {
            inner: self.inner.clone(),
        }
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

    fn insert(&self, key: Key, value: Bound<'_, PyAny>) -> HashTrieMapPy {
        HashTrieMapPy {
            inner: self.inner.insert(key, value.unbind()),
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
    fn update(
        &self,
        maps: &Bound<'_, PyTuple>,
        kwds: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<HashTrieMapPy> {
        let mut inner = self.inner.clone();
        for value in maps {
            let map = HashTrieMapPy::extract(value.as_borrowed())?;
            for (k, v) in &map.inner {
                inner.insert_mut(k.clone_ref(value.py()), v.clone_ref(value.py()));
            }
        }
        if let Some(kwds) = kwds {
            for (k, v) in kwds {
                inner.insert_mut(Key::extract(k.as_borrowed())?, v.extract()?);
            }
        }
        Ok(HashTrieMapPy { inner })
    }
}

#[pyclass(module = "rpds")]
struct KeysIterator {
    inner: HashTrieMapSync<Key, Py<PyAny>>,
}

#[pymethods]
impl KeysIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Key> {
        let first = slf.inner.keys().next()?.clone_ref(slf.py());
        slf.inner = slf.inner.remove(&first);
        Some(first)
    }
}

#[pyclass(module = "rpds")]
struct ValuesIterator {
    inner: HashTrieMapSync<Key, Py<PyAny>>,
}

#[pymethods]
impl ValuesIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Py<PyAny>> {
        let kv = slf.inner.iter().next()?;
        let value = kv.1.clone_ref(slf.py());
        slf.inner = slf.inner.remove(kv.0);
        Some(value)
    }
}

#[pyclass(module = "rpds")]
struct ItemsIterator {
    inner: HashTrieMapSync<Key, Py<PyAny>>,
}

#[pymethods]
impl ItemsIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(Key, Py<PyAny>)> {
        let kv = slf.inner.iter().next()?;
        let key = kv.0.clone_ref(slf.py());
        let value = kv.1.clone_ref(slf.py());

        slf.inner = slf.inner.remove(kv.0);

        Some((key, value))
    }
}

#[pyclass(module = "rpds")]
struct KeysView {
    inner: HashTrieMapSync<Key, Py<PyAny>>,
}

#[pymethods]
impl KeysView {
    fn __contains__(&self, key: Key) -> bool {
        self.inner.contains_key(&key)
    }

    fn __eq__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? != slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            if !slf.inner.contains_key(&Key::extract(each?.as_borrowed())?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __lt__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? <= slf.inner.size() {
            return Ok(false);
        }

        for each in slf.inner.keys() {
            if !other.contains(each.inner.clone_ref(slf.py()))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __le__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? < slf.inner.size() {
            return Ok(false);
        }

        for each in slf.inner.keys() {
            if !other.contains(each.inner.clone_ref(slf.py()))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __gt__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? >= slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            if !slf.inner.contains_key(&Key::extract(each?.as_borrowed())?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __ge__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? > slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            if !slf.inner.contains_key(&Key::extract(each?.as_borrowed())?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> KeysIterator {
        KeysIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __len__(slf: PyRef<'_, Self>) -> usize {
        slf.inner.size()
    }

    fn __and__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>) -> PyResult<HashTrieSetPy> {
        KeysView::intersection(slf, other)
    }

    fn __or__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<KeysView> {
        KeysView::union(slf, other, py)
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|(k, _)| {
            Ok(k.clone_ref(py)
                .inner
                .into_pyobject(py)?
                .call_method0("__repr__")
                .and_then(|r| r.extract())
                .unwrap_or("<repr failed>".to_owned()))
        });
        let contents = contents.collect::<Result<Vec<_>, PyErr>>()?;
        Ok(format!("keys_view({{{}}})", contents.join(", ")))
    }

    fn intersection(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>) -> PyResult<HashTrieSetPy> {
        // TODO: iterate over the shorter one if it's got a length
        let mut inner = HashTrieSet::new_sync();
        for each in other.try_iter()? {
            let key = Key::extract(each?.as_borrowed())?;
            if slf.inner.contains_key(&key) {
                inner.insert_mut(key);
            }
        }
        Ok(HashTrieSetPy { inner })
    }

    fn union(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<KeysView> {
        // There doesn't seem to be a low-effort way to get a HashTrieSet out of a map,
        // so we just keep our map and add values we'll ignore.
        let mut inner = slf.inner.clone();
        for each in other.try_iter()? {
            inner.insert_mut(Key::extract(each?.as_borrowed())?, py.None());
        }
        Ok(KeysView { inner })
    }
}

#[pyclass(module = "rpds")]
struct ValuesView {
    inner: HashTrieMapSync<Key, Py<PyAny>>,
}

#[pymethods]
impl ValuesView {
    fn __iter__(slf: PyRef<'_, Self>) -> ValuesIterator {
        ValuesIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __len__(slf: PyRef<'_, Self>) -> usize {
        slf.inner.size()
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|(_, v)| {
            Ok(v.into_pyobject(py)?
                .call_method0("__repr__")
                .and_then(|r| r.extract())
                .unwrap_or("<repr failed>".to_owned()))
        });
        let contents = contents.collect::<Result<Vec<_>, PyErr>>()?;
        Ok(format!("values_view([{}])", contents.join(", ")))
    }
}

#[pyclass(module = "rpds")]
struct ItemsView {
    inner: HashTrieMapSync<Key, Py<PyAny>>,
}

#[derive(FromPyObject)]
struct ItemViewQuery(Key, Py<PyAny>);

#[pymethods]
impl ItemsView {
    fn __contains__(slf: PyRef<'_, Self>, item: ItemViewQuery) -> PyResult<bool> {
        if let Some(value) = slf.inner.get(&item.0) {
            return item.1.bind(slf.py()).eq(value);
        }

        Ok(false)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> ItemsIterator {
        ItemsIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __len__(slf: PyRef<'_, Self>) -> usize {
        slf.inner.size()
    }

    fn __eq__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? != slf.inner.size() {
            return Ok(false);
        }
        for (k, v) in slf.inner.iter() {
            if !other.contains((k.inner.clone_ref(slf.py()), v))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|(k, v)| {
            let tuple = PyTuple::new(py, [k.inner.clone_ref(py), v.clone_ref(py)])?;
            Ok(format!("{:?}", tuple))
        });
        let contents = contents.collect::<Result<Vec<_>, PyErr>>()?;
        Ok(format!("items_view([{}])", contents.join(", ")))
    }

    fn __lt__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? <= slf.inner.size() {
            return Ok(false);
        }
        for (k, v) in slf.inner.iter() {
            let pair = PyTuple::new(py, [k.inner.clone_ref(py), v.clone_ref(py)])?;
            // FIXME: needs to compare
            if !other.contains(pair)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __le__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? < slf.inner.size() {
            return Ok(false);
        }
        for (k, v) in slf.inner.iter() {
            let pair = PyTuple::new(py, [k.inner.clone_ref(py), v.clone_ref(py)])?;
            // FIXME: needs to compare
            if !other.contains(pair)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __gt__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? >= slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            let kv = each?;
            let k = kv.get_item(0)?;
            match slf.inner.get(&Key::extract(k.as_borrowed())?) {
                Some(value) => {
                    let pair = PyTuple::new(py, [k, value.bind(py).clone()])?;
                    if !pair.eq(kv)? {
                        return Ok(false);
                    }
                }
                None => return Ok(false),
            }
        }
        Ok(true)
    }

    fn __ge__(slf: PyRef<'_, Self>, other: &Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? > slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            let kv = each?;
            let k = kv.get_item(0)?;
            match slf.inner.get(&Key::extract(k.as_borrowed())?) {
                Some(value) => {
                    let pair = PyTuple::new(py, [k, value.bind(py).clone()])?;
                    if !pair.eq(kv)? {
                        return Ok(false);
                    }
                }
                None => return Ok(false),
            }
        }
        Ok(true)
    }

    fn __and__(
        slf: PyRef<'_, Self>,
        other: &Bound<'_, PyAny>,
        py: Python,
    ) -> PyResult<HashTrieSetPy> {
        ItemsView::intersection(slf, other, py)
    }

    fn __or__(
        slf: PyRef<'_, Self>,
        other: &Bound<'_, PyAny>,
        py: Python,
    ) -> PyResult<HashTrieSetPy> {
        ItemsView::union(slf, other, py)
    }

    fn intersection(
        slf: PyRef<'_, Self>,
        other: &Bound<'_, PyAny>,
        py: Python,
    ) -> PyResult<HashTrieSetPy> {
        // TODO: iterate over the shorter one if it's got a length
        let mut inner = HashTrieSet::new_sync();
        for each in other.try_iter()? {
            let kv = each?;
            let k = kv.get_item(0)?;
            if let Some(value) = slf.inner.get(&Key::extract(k.as_borrowed())?) {
                let pair = PyTuple::new(py, [k, value.bind(py).clone()])?;
                if pair.eq(kv)? {
                    inner.insert_mut(Key::extract(pair.as_any().as_borrowed())?);
                }
            }
        }
        Ok(HashTrieSetPy { inner })
    }

    fn union(
        slf: PyRef<'_, Self>,
        other: &Bound<'_, PyAny>,
        py: Python,
    ) -> PyResult<HashTrieSetPy> {
        // TODO: this is very inefficient, but again can't seem to get a HashTrieSet out of ourself
        let mut inner = HashTrieSet::new_sync();
        for (k, v) in slf.inner.iter() {
            let pair = PyTuple::new(py, [k.inner.clone_ref(py), v.clone_ref(py)])?;
            inner.insert_mut(Key::extract(pair.as_any().as_borrowed())?);
        }
        for each in other.try_iter()? {
            inner.insert_mut(Key::extract(each?.as_borrowed())?);
        }
        Ok(HashTrieSetPy { inner })
    }
}

#[repr(transparent)]
#[pyclass(name = "HashTrieSet", module = "rpds", frozen)]
struct HashTrieSetPy {
    inner: HashTrieSetSync<Key>,
}

impl<'py> FromPyObject<'_, 'py> for HashTrieSetPy {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let mut ret = HashTrieSet::new_sync();
        for each in ob.try_iter()? {
            let k: Key = each?.extract()?;
            ret.insert_mut(k);
        }
        Ok(HashTrieSetPy { inner: ret })
    }
}

#[pymethods]
impl HashTrieSetPy {
    #[new]
    #[pyo3(signature = (value=None))]
    fn init(value: Option<HashTrieSetPy>) -> Self {
        if let Some(value) = value {
            value
        } else {
            HashTrieSetPy {
                inner: HashTrieSet::new_sync(),
            }
        }
    }

    fn __contains__(&self, key: Key) -> bool {
        self.inner.contains(&key)
    }

    fn __and__(&self, other: &Self, py: Python) -> Self {
        self.intersection(other, py)
    }

    fn __or__(&self, other: &Self, py: Python) -> Self {
        self.union(other, py)
    }

    fn __sub__(&self, other: &Self) -> Self {
        self.difference(other)
    }

    fn __xor__(&self, other: &Self, py: Python) -> Self {
        self.symmetric_difference(other, py)
    }

    fn __iter__(slf: PyRef<'_, Self>) -> SetIterator {
        SetIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.size()
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|k| {
            Ok(k.clone_ref(py)
                .into_pyobject(py)?
                .call_method0("__repr__")
                .and_then(|r| r.extract())
                .unwrap_or("<repr failed>".to_owned()))
        });
        let contents = contents.collect::<Result<Vec<_>, PyErr>>()?;
        Ok(format!("HashTrieSet({{{}}})", contents.join(", ")))
    }

    fn __eq__(slf: PyRef<'_, Self>, other: Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? != slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            if !slf.inner.contains(&Key::extract(each?.as_borrowed())?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __hash__(&self) -> PyResult<isize> {
        // modified from https://github.com/python/cpython/blob/d69529d31ccd1510843cfac1ab53bb8cb027541f/Objects/setobject.c#L715

        let mut hash_val = self
            .inner
            .iter()
            .map(|k| k.hash as usize)
            .fold(0, |acc: usize, x: usize| acc ^ hash_shuffle_bits(x));

        // factor in the number of entries in the collection
        hash_val ^= self.inner.size().wrapping_add(1).wrapping_mul(1927868237);

        // dispense patterns in the hash value
        hash_val ^= (hash_val >> 11) ^ (hash_val >> 25);
        hash_val = hash_val.wrapping_mul(69069).wrapping_add(907133923);

        Ok(hash_val as isize)
    }

    fn __lt__(slf: PyRef<'_, Self>, other: Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? <= slf.inner.size() {
            return Ok(false);
        }
        for each in slf.inner.iter() {
            if !other.contains(each.inner.clone_ref(py))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __le__(slf: PyRef<'_, Self>, other: Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? < slf.inner.size() {
            return Ok(false);
        }
        for each in slf.inner.iter() {
            if !other.contains(each.inner.clone_ref(slf.py()))? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __gt__(slf: PyRef<'_, Self>, other: Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? >= slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            if !slf.inner.contains(&Key::extract(each?.as_borrowed())?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __ge__(slf: PyRef<'_, Self>, other: Bound<'_, PyAny>, py: Python) -> PyResult<bool> {
        let abc = PyModule::import(py, "collections.abc")?;
        if !other.is_instance(&abc.getattr("Set")?)? || other.len()? > slf.inner.size() {
            return Ok(false);
        }
        for each in other.try_iter()? {
            if !slf.inner.contains(&Key::extract(each?.as_borrowed())?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn __reduce__(slf: PyRef<'_, Self>) -> (Bound<'_, PyType>, (Vec<Key>,)) {
        (
            HashTrieSetPy::type_object(slf.py()),
            (slf.inner.iter().map(|e| e.clone_ref(slf.py())).collect(),),
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

    fn intersection(&self, other: &Self, py: Python) -> HashTrieSetPy {
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
                inner.insert_mut(value.clone_ref(py));
            }
        }
        HashTrieSetPy { inner }
    }

    fn symmetric_difference(&self, other: &Self, py: Python) -> HashTrieSetPy {
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
                inner.insert_mut(value.clone_ref(py));
            }
        }
        HashTrieSetPy { inner }
    }

    fn union(&self, other: &Self, py: Python) -> HashTrieSetPy {
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
            inner.insert_mut(value.clone_ref(py));
        }
        HashTrieSetPy { inner }
    }

    #[pyo3(signature = (*iterables))]
    fn update(&self, iterables: Bound<'_, PyTuple>) -> PyResult<HashTrieSetPy> {
        let mut inner = self.inner.clone();
        for each in iterables {
            let iter = each.try_iter()?;
            for value in iter {
                inner.insert_mut(Key::extract(value?.as_borrowed())?);
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
        let first = slf.inner.iter().next()?.clone_ref(slf.py());
        slf.inner = slf.inner.remove(&first);
        Some(first)
    }
}

#[repr(transparent)]
#[pyclass(name = "List", module = "rpds", frozen, sequence)]
struct ListPy {
    inner: ListSync<Py<PyAny>>,
}

impl From<ListSync<Py<PyAny>>> for ListPy {
    fn from(elements: ListSync<Py<PyAny>>) -> Self {
        ListPy { inner: elements }
    }
}

impl<'py> FromPyObject<'_, 'py> for ListPy {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let mut ret = List::new_sync();
        let reversed = PyModule::import(ob.py(), "builtins")?.getattr("reversed")?;
        let rob: Bound<'_, PyIterator> = reversed.call1((ob,))?.try_iter()?;
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
    fn init(elements: &Bound<'_, PyTuple>) -> PyResult<Self> {
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

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|k| {
            Ok(k.into_pyobject(py)?
                .call_method0("__repr__")
                .and_then(|r| r.extract())
                .unwrap_or("<repr failed>".to_owned()))
        });
        let contents = contents.collect::<Result<Vec<_>, PyErr>>()?;
        Ok(format!("List([{}])", contents.join(", ")))
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match op {
            CompareOp::Eq => (self.inner.len() == other.inner.len()
                && self
                    .inner
                    .iter()
                    .zip(other.inner.iter())
                    .map(|(e1, e2)| e1.bind(py).eq(e2))
                    .all(|r| r.unwrap_or(false)))
            .into_pyobject(py)
            .map_err(Into::into)
            .map(BoundObject::into_any)
            .map(BoundObject::unbind),
            CompareOp::Ne => (self.inner.len() != other.inner.len()
                || self
                    .inner
                    .iter()
                    .zip(other.inner.iter())
                    .map(|(e1, e2)| e1.bind(py).ne(e2))
                    .any(|r| r.unwrap_or(true)))
            .into_pyobject(py)
            .map_err(Into::into)
            .map(BoundObject::into_any)
            .map(BoundObject::unbind),
            _ => Ok(py.NotImplemented()),
        }
    }

    fn __hash__(&self, py: Python) -> PyResult<u64> {
        let mut hasher = DefaultHasher::new();

        self.inner
            .iter()
            .enumerate()
            .try_for_each(|(index, each)| {
                each.bind(py)
                    .hash()
                    .map_err(|_| {
                        PyTypeError::new_err(format!(
                            "Unhashable type at {} element in List: {}",
                            index,
                            each.bind(py)
                                .repr()
                                .and_then(|r| r.extract())
                                .unwrap_or("<repr> error".to_string())
                        ))
                    })
                    .map(|x| hasher.write_isize(x))
            })?;

        Ok(hasher.finish())
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

    fn __reduce__(slf: PyRef<'_, Self>) -> (Bound<'_, PyType>, (Vec<Py<PyAny>>,)) {
        (
            ListPy::type_object(slf.py()),
            (slf.inner.iter().map(|e| e.clone_ref(slf.py())).collect(),),
        )
    }

    #[getter]
    fn first(&self) -> PyResult<&Py<PyAny>> {
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

    fn push_front(&self, other: Py<PyAny>) -> ListPy {
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
    inner: ListSync<Py<PyAny>>,
}

#[pymethods]
impl ListIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Py<PyAny>> {
        let first_op = slf.inner.first()?;
        let first = first_op.clone_ref(slf.py());

        slf.inner = slf.inner.drop_first()?;

        Some(first)
    }
}

#[repr(transparent)]
#[pyclass(name = "Stack", module = "rpds", frozen, sequence)]
struct StackPy {
    inner: StackSync<Py<PyAny>>,
}

impl From<StackSync<Py<PyAny>>> for StackPy {
    fn from(elements: StackSync<Py<PyAny>>) -> Self {
        StackPy { inner: elements }
    }
}

#[pymethods]
impl StackPy {
    #[new]
    #[pyo3(signature = (*args))]
    fn init(args: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let mut inner = Stack::new_sync();
        if args.len() == 1 {
            for each in args.get_item(0)?.try_iter()? {
                inner.push_mut(each?.extract()?);
            }
        } else {
            for each in args {
                inner.push_mut(each.extract()?);
            }
        }
        Ok(StackPy { inner })
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<u64> {
        let mut hasher = DefaultHasher::new();

        self.inner
            .iter()
            .enumerate()
            .try_for_each(|(index, each)| {
                each.bind(py)
                    .hash()
                    .map_err(|_| {
                        PyTypeError::new_err(format!(
                            "Unhashable type at {} element in Stack: {}",
                            index,
                            each.bind(py)
                                .repr()
                                .and_then(|r| r.extract())
                                .unwrap_or("<repr> error".to_string())
                        ))
                    })
                    .map(|x| hasher.write_isize(x))
            })?;

        Ok(hasher.finish())
    }

    fn __iter__(slf: PyRef<'_, Self>) -> StackIterator {
        StackIterator {
            inner: slf.inner.clone(),
        }
    }

    fn __len__(&self) -> usize {
        self.inner.size()
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match op {
            CompareOp::Eq => (self.inner.size() == other.inner.size()
                && self
                    .inner
                    .iter()
                    .zip(other.inner.iter())
                    .map(|(e1, e2)| e1.bind(py).eq(e2))
                    .all(|r| r.unwrap_or(false)))
            .into_pyobject(py)
            .map_err(Into::into)
            .map(BoundObject::into_any)
            .map(BoundObject::unbind),
            CompareOp::Ne => (self.inner.size() != other.inner.size()
                || self
                    .inner
                    .iter()
                    .zip(other.inner.iter())
                    .map(|(e1, e2)| e1.bind(py).ne(e2))
                    .any(|r| r.unwrap_or(true)))
            .into_pyobject(py)
            .map_err(Into::into)
            .map(BoundObject::into_any)
            .map(BoundObject::unbind),
            _ => Ok(py.NotImplemented()),
        }
    }

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|k| {
            Ok(k.into_pyobject(py)?
                .call_method0("__repr__")
                .and_then(|r| r.extract())
                .unwrap_or("<repr failed>".to_owned()))
        });
        let mut contents = contents.collect::<Result<Vec<_>, PyErr>>()?;
        contents.reverse();
        Ok(format!("Stack([{}])", contents.join(", ")))
    }

    fn peek(&self, py: Python) -> PyResult<Py<PyAny>> {
        if let Some(peeked) = self.inner.peek() {
            Ok(peeked.clone_ref(py))
        } else {
            Err(PyIndexError::new_err("peeked an empty stack"))
        }
    }

    fn pop(&self) -> PyResult<StackPy> {
        if let Some(popped) = self.inner.pop() {
            Ok(StackPy { inner: popped })
        } else {
            Err(PyIndexError::new_err("popped an empty stack"))
        }
    }

    fn push(&self, other: Py<PyAny>) -> StackPy {
        StackPy {
            inner: self.inner.push(other),
        }
    }
}

#[pyclass(module = "rpds")]
struct StackIterator {
    inner: StackSync<Py<PyAny>>,
}

#[pymethods]
impl StackIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Py<PyAny>> {
        let first_op = slf.inner.peek()?;
        let first = first_op.clone_ref(slf.py());

        slf.inner = slf.inner.pop()?;

        Some(first)
    }
}

#[pyclass(module = "rpds")]
struct QueueIterator {
    inner: QueueSync<Py<PyAny>>,
}

#[pymethods]
impl QueueIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<Py<PyAny>> {
        let first_op = slf.inner.peek()?;
        let first = first_op.clone_ref(slf.py());
        slf.inner = slf.inner.dequeue()?;
        Some(first)
    }
}

#[repr(transparent)]
#[pyclass(name = "Queue", module = "rpds", frozen, sequence)]
struct QueuePy {
    inner: QueueSync<Py<PyAny>>,
}

impl From<QueueSync<Py<PyAny>>> for QueuePy {
    fn from(elements: QueueSync<Py<PyAny>>) -> Self {
        QueuePy { inner: elements }
    }
}

impl<'py> FromPyObject<'_, 'py> for QueuePy {
    type Error = PyErr;

    fn extract(ob: Borrowed<'_, 'py, PyAny>) -> Result<Self, Self::Error> {
        let mut ret = Queue::new_sync();
        for each in ob.try_iter()? {
            ret.enqueue_mut(each?.extract()?);
        }
        Ok(QueuePy { inner: ret })
    }
}

#[pymethods]
impl QueuePy {
    #[new]
    #[pyo3(signature = (*elements))]
    fn init(elements: &Bound<'_, PyTuple>, py: Python<'_>) -> PyResult<Self> {
        let mut ret: QueuePy;
        if elements.len() == 1 {
            ret = elements.get_item(0)?.extract()?;
        } else {
            ret = QueuePy {
                inner: Queue::new_sync(),
            };
            if elements.len() > 1 {
                for each in elements {
                    ret.inner.enqueue_mut(each.into_pyobject(py)?.unbind());
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
                .map(|(e1, e2)| e1.bind(py).eq(e2))
                .all(|r| r.unwrap_or(false))
    }

    fn __hash__(&self, py: Python<'_>) -> PyResult<u64> {
        let mut hasher = DefaultHasher::new();

        self.inner
            .iter()
            .enumerate()
            .try_for_each(|(index, each)| {
                each.bind(py)
                    .hash()
                    .map_err(|_| {
                        PyTypeError::new_err(format!(
                            "Unhashable type at {} element in Queue: {}",
                            index,
                            each.bind(py)
                                .repr()
                                .and_then(|r| r.extract())
                                .unwrap_or("<repr> error".to_string())
                        ))
                    })
                    .map(|x| hasher.write_isize(x))
            })?;

        Ok(hasher.finish())
    }

    fn __ne__(&self, other: &Self, py: Python<'_>) -> bool {
        (self.inner.len() != other.inner.len())
            || self
                .inner
                .iter()
                .zip(other.inner.iter())
                .map(|(e1, e2)| e1.bind(py).ne(e2))
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

    fn __repr__(&self, py: Python) -> PyResult<String> {
        let contents = self.inner.into_iter().map(|k| {
            Ok(k.into_pyobject(py)?
                .call_method0("__repr__")
                .and_then(|r| r.extract())
                .unwrap_or("<repr failed>".to_owned()))
        });
        let contents = contents.collect::<Result<Vec<_>, PyErr>>()?;
        Ok(format!("Queue([{}])", contents.join(", ")))
    }

    fn peek(&self, py: Python) -> PyResult<Py<PyAny>> {
        if let Some(peeked) = self.inner.peek() {
            Ok(peeked.clone_ref(py))
        } else {
            Err(PyIndexError::new_err("peeked an empty queue"))
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn enqueue(&self, value: Bound<'_, PyAny>) -> Self {
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

#[pymodule(gil_used = false)]
#[pyo3(name = "rpds")]
fn rpds_py(py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HashTrieMapPy>()?;
    m.add_class::<HashTrieSetPy>()?;
    m.add_class::<ListPy>()?;
    m.add_class::<StackPy>()?;
    m.add_class::<QueuePy>()?;

    PyMapping::register::<HashTrieMapPy>(py)?;

    let abc = PyModule::import(py, "collections.abc")?;

    abc.getattr("Set")?
        .call_method1("register", (HashTrieSetPy::type_object(py),))?;

    abc.getattr("MappingView")?
        .call_method1("register", (KeysView::type_object(py),))?;
    abc.getattr("MappingView")?
        .call_method1("register", (ValuesView::type_object(py),))?;
    abc.getattr("MappingView")?
        .call_method1("register", (ItemsView::type_object(py),))?;

    abc.getattr("KeysView")?
        .call_method1("register", (KeysView::type_object(py),))?;
    abc.getattr("ValuesView")?
        .call_method1("register", (ValuesView::type_object(py),))?;
    abc.getattr("ItemsView")?
        .call_method1("register", (ItemsView::type_object(py),))?;

    Ok(())
}
