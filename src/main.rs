use wasmtime::*;

fn main() -> anyhow::Result<()> {
    // Create an engine with threads support
    let mut config = Config::new();
    config.wasm_threads(true);
    let engine = Engine::new(&config)?;

    // Modified WebAssembly module with shared and private memory
    let module_wat = r#"
        (module
            ;; Shared memory that can be imported or created
            (memory (import "env" "memory") 1 1 shared)
            
            ;; Private counter (like private namespace)
            (global $private_counter (mut i32) (i32.const 0))
            
            ;; Shared data goes into memory at offset 0
            (func (export "increment_shared") (result i32)
                (local $temp i32)
                ;; Atomically increment and get the old value
                (i32.atomic.rmw.add
                    (i32.const 0)
                    (i32.const 1)
                )
            )
            
            ;; Private counter operations
            (func (export "increment_private") (result i32)
                (local $temp i32)
                global.get $private_counter
                i32.const 1
                i32.add
                local.tee $temp
                global.set $private_counter
                local.get $temp
            )
        )
    "#;

    let module = Module::new(&engine, module_wat)?;

    // Create a store
    let mut store = Store::new(&engine, ());

    // Create a new memory instance to share with atomic support
    let memory = Memory::new(&mut store, MemoryType::shared(1, 1))?;

    // Create the import object with our memory
    let imports = [
        Extern::Memory(memory),
    ];

    // Create both instances with the shared memory
    let instance1 = Instance::new(&mut store, &module, &imports)?;
    let instance2 = Instance::new(&mut store, &module, &imports)?;

    // Get functions from both instances
    let shared1 = instance1.get_typed_func::<(), i32>(&mut store, "increment_shared")?;
    let private1 = instance1.get_typed_func::<(), i32>(&mut store, "increment_private")?;
    
    let shared2 = instance2.get_typed_func::<(), i32>(&mut store, "increment_shared")?;
    let private2 = instance2.get_typed_func::<(), i32>(&mut store, "increment_private")?;

    // Demonstrate shared vs private state
    println!("Instance 1 shared count: {}", shared1.call(&mut store, ())?);   // Prints 1
    println!("Instance 1 shared count again: {}", shared1.call(&mut store, ())?);   // Should print 2
    println!("Instance 2 shared count: {}", shared2.call(&mut store, ())?);   // Should print 3
    println!("\nTesting private counters:");
    println!("Instance 1 private count: {}", private1.call(&mut store, ())?); // Prints 1
    println!("Instance 1 private count again: {}", private1.call(&mut store, ())?); // Prints 2
    println!("Instance 2 private count: {}", private2.call(&mut store, ())?); // Prints 1 (separate)

    Ok(())
}

