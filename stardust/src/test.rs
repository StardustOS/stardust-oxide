use {
    alloc::{format, vec::Vec},
    log::{debug, error},
    xen::xenstore,
};

const TESTS: [&dyn Fn(); 2] = [&allocator, &xenstore];

pub fn tests() {
    error!("RUNNING {} TESTS", TESTS.len());
    for test in TESTS {
        test();
    }
}

fn allocator() {
    {
        let mut a = Vec::with_capacity(30_000_000);
        for i in 0..30_000_000 {
            a.push((i % 256) as u8);
        }
        for i in (0..30_000_000).rev() {
            assert_eq!(a.pop().unwrap(), (i % 256) as u8);
        }
        assert_eq!(a.len(), 0);
    }

    let mut a = Vec::with_capacity(500_000);
    for i in 0..500_000 {
        let str = format!("string number {}", i);
        a.push(str);
    }
    assert_eq!(a.last().unwrap().len(), 20);
}

fn xenstore() {
    xenstore::write(
        format!("/local/domain/{}/data\0", xenstore::domain_id()),
        format!("hello from domain {}!\0", xenstore::domain_id()),
    );

    debug!(
        "local domain contents: {:?}",
        xenstore::ls(format!("/local/domain/{}\0", xenstore::domain_id()))
    );

    debug!(
        "test: {:?}",
        xenstore::read(format!("/local/domain/{}/data\0", xenstore::domain_id()))
    );
}
