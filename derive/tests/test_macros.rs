#[cfg(feature = "trace-writer")]
use p3_derive::AirColumns;

#[test]
#[cfg(feature = "trace-writer")]
fn test_generics() {
    #[derive(AirColumns)]
    struct A<T> {
        _a: T,
    }

    assert_eq!(A::<u32>::headers(), vec!["_a"]);
}
#[test]
#[cfg(feature = "trace-writer")]
fn test_generic_array() {
    #[derive(AirColumns)]
    struct A<T> {
        _a: [T; 1],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0]"]);
}

#[test]
#[cfg(feature = "trace-writer")]
fn test_generic_nested_array() {
    #[derive(AirColumns)]
    struct A<T> {
        _a: [[T; 1]; 1],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0][0]"]);
}

#[test]
#[cfg(feature = "trace-writer")]
fn test_array_variable_length() {
    const N: usize = 1;

    #[derive(AirColumns)]
    struct A<T> {
        _a: [T; N],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0]"]);
}

#[test]
#[cfg(feature = "trace-writer")]
fn test_nested_array_variable_length() {
    const N: usize = 1;

    #[derive(AirColumns)]
    struct A<T> {
        _a: [[T; N]; N],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0][0]"]);
}
