use p3_derive::AirColumns;

#[test]
fn test_generics() {
    #[derive(AirColumns)]
    struct A<T> {
        _a: T,
    }

    assert_eq!(A::<u32>::headers(), vec!["_a"]);
}
#[test]
fn test_generic_array() {
    #[derive(AirColumns)]
    struct A<T> {
        _a: [T; 1],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0]"]);
}

#[test]
fn test_generic_nested_array() {
    #[derive(AirColumns)]
    struct A<T> {
        _a: [[T; 1]; 1],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0][0]"]);
}

#[test]
fn test_array_variable_length() {
    const N: usize = 1;

    #[derive(AirColumns)]
    struct A<T> {
        _a: [T; N],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0]"]);
}

#[test]
fn test_nested_array_variable_length() {
    const N: usize = 1;

    #[derive(AirColumns)]
    struct A<T> {
        _a: [[T; N]; N],
    }

    assert_eq!(A::<u32>::headers(), vec!["_a[0][0]"]);
}
