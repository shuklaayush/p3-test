use p3_derive::Headers;

#[test]
fn test_simple() {
    #[derive(Headers)]
    struct A {
        a: u32,
        b: u32,
    }

    assert_eq!(A::headers(), vec!["a", "b"]);
}

#[test]
fn test_array() {
    #[derive(Headers)]
    struct A {
        a: [u32; 1],
    }

    assert_eq!(A::headers(), vec!["a[0]"]);
}

#[test]
fn test_nested_array() {
    #[derive(Headers)]
    struct A {
        a: [[u32; 1]; 1],
    }

    assert_eq!(A::headers(), vec!["a[0][0]"]);
}

#[test]
fn test_generics() {
    #[derive(Headers)]
    struct A<T> {
        a: T,
    }

    assert_eq!(A::<u32>::headers(), vec!["a"]);
}
#[test]
fn test_generic_array() {
    #[derive(Headers)]
    struct A<T> {
        a: [T; 1],
    }

    assert_eq!(A::<u32>::headers(), vec!["a[0]"]);
}

#[test]
fn test_generic_nested_array() {
    #[derive(Headers)]
    struct A<T> {
        a: [[T; 1]; 1],
    }

    assert_eq!(A::<u32>::headers(), vec!["a[0][0]"]);
}

#[test]
fn test_array_variable_length() {
    const N: usize = 1;

    #[derive(Headers)]
    struct A {
        a: [u32; N],
    }

    assert_eq!(A::headers(), vec!["a[0]"]);
}

#[test]
fn test_nested_array_variable_length() {
    const N: usize = 1;

    #[derive(Headers)]
    struct A {
        a: [[u32; N]; N],
    }

    assert_eq!(A::headers(), vec!["a[0][0]"]);
}
