/**
 * @file Mochi grammar for tree-sitter
 * @author Hugo Darrieutort-Garcia <h.darrieutortg@gmail.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

const PREC = {
  assign: 1,
  equals: 2,
  compare: 3,
  sum: 4,
  product: 5,
  prefix: 6,
  question: 7,
  call: 8,
  field: 9,
};

module.exports = grammar({
  name: 'mochi',

  extras: $ => [
    $.comment,
    /\s+/,
  ],

  word: $ => $.identifier,

  conflicts: $ => [
    [$.if_expression, $.block_expression],
    [$.atom, $.custom_type],
  ],

  rules: {
    source_file: $ => repeat($._expression),

    _expression: $ => choice(
      $.identifier,
      $.number,
      $.string,
      $.atom,
      $.array,
      $.hash,
      $.tuple,
      $.prefix_expression,
      $.infix_expression,
      $.if_expression,
      $.function_definition,
      $.block_expression,
      $.let_expression,
      $.const_expression,
      $.assignment_expression,
      $.member_expression,
      $.index_expression,
      $.method_call,
      $.call_expression,
      $.loop_expression,
      $.while_expression,
      $.for_expression,
      $.match_expression,
      $.question_expression,
      $.break_expression,
      $.continue_expression,
      $.import_expression,
      $.return_expression,
      $.parenthesized_expression,
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    number: $ => /\d+(\.\d+)?/,

    string: $ => /"[^"\\]*(\\.[^"\\]*)*"/,

    atom: $ => seq(':', $.identifier),

    comment: $ => /--.*/,

    prefix_expression: $ => prec(PREC.prefix, seq(
      choice('!', '-'),
      field('right', $._expression),
    )),

    infix_expression: $ => choice(
      prec.left(PREC.product, seq($._expression, choice('*', '/'), $._expression)),
      prec.left(PREC.sum, seq($._expression, choice('+', '-'), $._expression)),
      prec.left(PREC.compare, seq($._expression, choice('>', '<'), $._expression)),
      prec.left(PREC.equals, seq($._expression, choice('==', '!='), $._expression)),
    ),

    if_expression: $ => choice(
      seq(
        'if',
        field('condition', $._expression),
        'do',
        field('consequence', repeat($._expression)),
        optional(seq('else', field('alternative', repeat($._expression)))),
        'end',
      ),
      prec.left(PREC.assign, seq(
        'if',
        field('condition', $._expression),
        field('consequence', $._expression),
        'else',
        field('alternative', $._expression),
      )),
    ),

    function_definition: $ => seq(
      'fn',
      optional(field('name', $.identifier)),
      '(',
      field('parameters', optional(commaSep($.parameter))),
      ')',
      optional(seq(':', field('return_type', $._type_annotation))),
      field('body', $._expression)
    ),

    parameter: $ => seq(
      field('name', $.identifier),
      optional(seq(':', field('type', $._type_annotation))),
    ),

    block_expression: $ => seq(
      'do',
      repeat($._expression),
      'end',
    ),

    let_expression: $ => seq(
      'let',
      field('name', $.identifier),
      optional(seq(':', field('type', $._type_annotation))),
      '=',
      field('value', $._expression),
    ),

    const_expression: $ => seq(
      'const',
      field('name', $.identifier),
      optional(seq(':', field('type', $._type_annotation))),
      '=',
      field('value', $._expression),
    ),

    assignment_expression: $ => prec.right(PREC.assign, seq(
      field('left', choice($.identifier, $.index_expression, $.member_expression)),
      '=',
      field('right', $._expression),
    )),

    member_expression: $ => prec(PREC.field, seq(
      field('object', $._expression),
      '.',
      field('property', $.identifier),
    )),

    index_expression: $ => prec(PREC.call, seq(
      field('object', $._expression),
      '[',
      field('index', $._expression),
      ']',
    )),

    method_call: $ => prec(PREC.call, seq(
      field('receiver', $.identifier),
      ':',
      field('method', $.identifier),
      '(',
      field('arguments', optional(commaSep($._expression))),
      ')',
    )),

    call_expression: $ => prec(PREC.call, seq(
      field('function', $._expression),
      '(',
      field('arguments', optional(commaSep($._expression))),
      ')',
    )),

    loop_expression: $ => seq(
      'loop',
      field('body', $._expression)
    ),

    while_expression: $ => seq(
      'while',
      field('condition', $._expression),
      field('body', $._expression)
    ),

    for_expression: $ => choice(
      seq(
        'for',
        field('key', $.identifier),
        ',',
        field('value', $.identifier),
        'in',
        field('iterable', $._expression),
        field('body', $._expression)
      ),
      seq(
        'for',
        field('element', $.identifier),
        'in',
        field('iterable', $._expression),
        field('body', $._expression)
      )
    ),

    match_expression: $ => prec.right(seq(
      'match',
      field('subject', $._expression),
      repeat1($.match_case)
    )),

    match_case: $ => seq(
      '|',
      field('pattern', $.pattern),
      optional(seq('when', field('guard', $._expression))),
      field('body', $._expression),
    ),

    pattern: $ => choice(
      $.wildcard_pattern,
      $.identifier,
      $.number,
      $.string,
      $.atom,
      $.tuple_pattern,
    ),

    wildcard_pattern: $ => '_',

    tuple_pattern: $ => seq(
      '(',
      optional(seq(commaSep1($.pattern), optional(','))),
      ')',
    ),

    question_expression: $ => prec(PREC.question, seq(
      field('expression', $._expression),
      '?',
    )),

    break_expression: $ => prec.right(PREC.prefix, seq(
      'break',
      optional(field('value', $._expression))
    )),

    continue_expression: $ => seq('continue'),

    import_expression: $ => prec(PREC.prefix, seq(
      'import',
      field('path', $._expression),
    )),

    return_expression: $ => prec(PREC.prefix, seq(
      'return',
      field('value', $._expression),
    )),

    parenthesized_expression: $ => seq(
      '(',
      $._expression,
      ')',
    ),

    array: $ => seq(
      '[',
      optional(commaSep1($._expression)),
      ']',
    ),

    hash: $ => seq(
      '{',
      optional(commaSep1($.hash_pair)),
      '}',
    ),

    hash_pair: $ => seq(
      field('key', $.identifier),
      ':',
      field('value', $._expression),
    ),

    tuple: $ => seq(
      '(',
      optional(choice(
        seq($._expression, ','),
        seq($._expression, repeat1(seq(',', $._expression)), optional(','))
      )),
      ')'
    ),

    _type_annotation: $ => choice(
      $.primitive_type,
      $.custom_type,
      $.array_type,
      $.hash_type,
      $.function_type,
      $.tuple_type,
    ),

    primitive_type: $ => choice('Number', 'String', 'Atom', 'Any'),

    custom_type: $ => $.identifier,

    array_type: $ => seq('[', $._type_annotation, ']'),

    hash_type: $ => seq(
      '{',
      optional(commaSep1($.hash_type_field)),
      '}',
    ),

    hash_type_field: $ => seq(
      field('key', $.identifier),
      ':',
      field('type', $._type_annotation),
    ),

    tuple_type: $ => seq(
      '(',
      optional(seq(commaSep1($._type_annotation), optional(','))),
      ')',
    ),

    function_type: $ => seq(
      'fn',
      '(',
      optional(commaSep1($.function_type_parameter)),
      ')',
      ':',
      field('return_type', $._type_annotation),
    ),

    function_type_parameter: $ => choice(
      $._type_annotation,
      seq($.identifier, ':', $._type_annotation),
    ),
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}
