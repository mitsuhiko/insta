use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct OrderedHashMap<K, V> {
    map: HashMap<K, V>,
    key_order: Vec<K>,
}

impl<K: Hash + Eq + Clone, V> OrderedHashMap<K, V> {
    pub fn new() -> Self {
        OrderedHashMap {
            map: HashMap::new(),
            key_order: Vec::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.map.entry(key) {
            Entry::Occupied(mut entry) => Some(entry.insert(value)),
            Entry::Vacant(entry) => {
                self.key_order.push(entry.key().clone());
                entry.insert(value);
                None
            }
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            map: &self.map,
            keys: self.key_order.iter(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<K: Hash + Eq, V: PartialEq> PartialEq for OrderedHashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.map == other.map
    }
}

impl<K: Hash + Eq, V: Eq> Eq for OrderedHashMap<K, V> {}

impl<K: Hash + Eq + PartialOrd + Clone, V: PartialOrd> PartialOrd for OrderedHashMap<K, V> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<K: Hash + Eq + Ord + Clone, V: Ord> Ord for OrderedHashMap<K, V> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<K: Hash + Eq + Clone, V: Hash> Hash for OrderedHashMap<K, V> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for item in self.iter() {
            item.hash(state);
        }
    }
}

impl<K: Hash + Eq + Clone, V> FromIterator<(K, V)> for OrderedHashMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = OrderedHashMap::new();
        for (k, v) in iter {
            map.insert(k, v);
        }
        map
    }
}

impl<K: Hash + Eq + Clone, V> IntoIterator for OrderedHashMap<K, V> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            map: self.map,
            keys: self.key_order.into_iter(),
        }
    }
}

pub struct IntoIter<K, V> {
    map: HashMap<K, V>,
    keys: std::vec::IntoIter<K>,
}

impl<K: Hash + Eq, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next().map(|key| {
            let value = self.map.remove(&key).unwrap();
            (key, value)
        })
    }
}

pub struct Iter<'a, K, V> {
    map: &'a HashMap<K, V>,
    keys: std::slice::Iter<'a, K>,
}

impl<'a, K: Hash + Eq, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next().map(|key| {
            let value = self.map.get(key).unwrap();
            (key, value)
        })
    }
}
