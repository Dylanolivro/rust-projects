mod cache;
use cache::{Cache, CacheConfig};

fn main() {
    let config = CacheConfig {
        max_capacity: 3,
        name: String::from("demo_cache"),
    };
    let mut cache = Cache::new(&config);
    cache.put("key1", 100);
    cache.put("key2", 200);
    cache.put("key3", 300);
    println!("Cache '{}' has {} items", cache.name(), cache.len());
    println!("Capacity: {}", cache.capacity());
    if let Some(value) = cache.get(&"key1") {
        println!("key1 = {}", value);
    }
    cache.put("key4", 400);
    println!("After inserting key4:");
    println!("Cache has {} items", cache.len());
    // key2 should have been evicted (LRU)
    if cache.contains(&"key2") {
        println!("key2 is still in cache (unexpected)");
    } else {
        println!("key2 was evicted (expected)");
    }
    if let Some(value) = cache.get(&"key3") {
        println!("key3 = {}", value);
    }
}
