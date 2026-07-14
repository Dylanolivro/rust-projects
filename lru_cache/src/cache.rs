pub struct CacheConfig {
    pub max_capacity: usize,
    pub name: String,
}
pub struct Cache<'a, K, V> {
    config: &'a CacheConfig,
    items: Vec<(K, V)>,
    access_order: Vec<usize>,
}
impl<'a, K, V> Cache<'a, K, V> {
    pub fn new(config: &'a CacheConfig) -> Self {
        Cache {
            config,
            items: Vec::new(),
            access_order: Vec::new(),
        }
    }
    pub fn capacity(&self) -> usize {
        self.config.max_capacity
    }
    pub fn name(&self) -> &str {
        &self.config.name
    }
    pub fn len(&self) -> usize {
        self.items.len()
    }
    pub fn contains(&mut self, key: &K) -> bool
    where
        K: PartialEq,
    {
        self.get(key).is_some()
    }
    pub fn get(&mut self, key: &K) -> Option<&V>
    where
        K: PartialEq,
    {
        let idx = self.items.iter().position(|(k, _)| k == key)?;
        self.bump(idx);
        Some(&self.items[idx].1)
    }
    pub fn put(&mut self, key: K, value: V)
    where
        K: PartialEq,
    {
        if let Some(idx) = self.items.iter().position(|(k, _)| *k == key) {
            self.items[idx].1 = value;
            self.bump(idx);
            return;
        }
        if self.items.len() >= self.capacity() {
            let evict_index = self.access_order.pop().unwrap();
            self.items.remove(evict_index);
            for idx in self.access_order.iter_mut() {
                if *idx > evict_index {
                    *idx -= 1;
                }
            }
        }
        self.items.push((key, value));
        self.access_order.insert(0, self.items.len() - 1);
    }
    fn bump(&mut self, index: usize) {
        let pos = self.access_order.iter().position(|&i| i == index).unwrap();
        self.access_order.remove(pos);
        self.access_order.insert(0, index);
    }
}
