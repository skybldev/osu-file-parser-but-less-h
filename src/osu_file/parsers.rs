use nom::{
    bytes::complete::{is_not, take_while},
    character::complete::char,
    character::complete::multispace0,
    combinator::map_res,
    error::ParseError,
    multi::{many0, separated_list0},
    sequence::{delimited, terminated, tuple},
    IResult,
};

// pub fn leading_ws<'a, F: 'a, O, E: ParseError<&'a str>>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Fn(&'a str) -> IResult<&'a str, O, E>,
// {
//     preceded(multispace0, inner)
// }

pub fn trailing_ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: Fn(&'a str) -> IResult<&'a str, O, E>,
{
    terminated(inner, multispace0)
}

pub fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
    inner: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
where
    F: FnMut(&'a str) -> IResult<&'a str, O, E>,
{
    delimited(multispace0, inner, multispace0)
}

pub fn get_colon_field_value_lines(s: &str) -> IResult<&str, Vec<(&str, &str)>> {
    let field_name = is_not::<_, _, nom::error::Error<_>>(": ");
    let field_separator = ws(char(':'));
    let field_value = is_not("\n\r");
    let field = tuple((
        terminated(field_name, field_separator),
        trailing_ws(field_value),
    ));

    many0(field)(s)
}

pub fn pipe_vec<'a, O, E, M, E2>(
    mapper: M,
) -> impl FnMut(&'a str) -> IResult<&'a str, Vec<O>, E>
where
    E: ParseError<&'a str> + nom::error::FromExternalError<&'a str, E2>,
    M: Fn(&str) -> Result<O, E2>,
{
    let pipe = char('|');
    let pipe_vec_item = take_while(|c: char| c != '|');
    let pipe_vec_item_map = map_res(pipe_vec_item, mapper);
    separated_list0(pipe, pipe_vec_item_map)
}
