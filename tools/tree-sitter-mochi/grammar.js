/**
 * @file Mochi grammar for tree-sitter
 * @author Hugo Darrieutort-Garcia <h.darrieutortg@gmail.com>
 * @license MIT
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: 'mochi',

  extras: $ => [
    /\s/,
    $.comment
  ],

  conflicts: $ => [
    [$.if_expression, $.block_expression]
  ],

  rules: {
    source_file: $ => repeat($._expression),

    comment: $ => /--.*/,

    _expression: $ => choice(
      $.identifier,
      $.number,
      $.string,
      $.symbol,
      $.let_expression,
      $.const_expression,
      $.assignment_expression,
      $.if_expression,
      $.function_expression,
      $.call_expression,
      $.unary_expression,
      $.binary_expression,
      $.block_expression,
      $.return_expression,
      $.array_literal,
      $.hash_literal,
      $.index_expression,
      $.dot_expression
    ),

    let_expression: $ => prec.right(2, seq(
      'let',
      field('name', $.identifier),
      '=',
      field('value', $._expression)
    )),

    const_expression: $ => prec.right(2, seq(
      'const',
      field('name', $.identifier),
      '=',
      field('value', $._expression)
    )),

    assignment_expression: $ => prec.right(2, seq(
      field('left', choice($.identifier, $.index_expression, $.dot_expression)),
      '=',
      field('value', $._expression)
    )),

    return_expression: $ => seq(
      'return',
      field('value', $._expression)
    ),

    block_expression: $ => seq(
      'do',
      repeat($._expression),
      'end'
    ),

    if_expression: $ => prec.right(seq(
      'if',
      field('condition', $._expression),
      choice(
        seq(
          'do',
          repeat($._expression),
          optional(seq('else', repeat($._expression))),
          'end'
        ),
        seq(
          field('consequence', $._expression),
          'else',
          field('alternative', $._expression)
        )
      )
    )),

    function_expression: $ => seq(
      'fn',
      optional(field('name', $.identifier)),
      '(',
      commaSep($.identifier),
      ')',
      field('body', $._expression)
    ),

    call_expression: $ => prec(8, seq(
      field('function', $._expression),
      '(',
      commaSep($._expression),
      ')'
    )),

    index_expression: $ => prec(8, seq(
      field('object', $._expression),
      '[',
      field('index', $._expression),
      ']'
    )),

    dot_expression: $ => prec(8, seq(
      field('object', $._expression),
      '.',
      field('property', $.identifier)
    )),

    array_literal: $ => seq(
      '[',
      commaSep($._expression),
      ']'
    ),

    hash_literal: $ => seq(
      '{',
      commaSep($.hash_pair),
      '}'
    ),

    hash_pair: $ => seq(
      field('key', $._expression),
      ':',
      field('value', $._expression)
    ),

    unary_expression: $ => prec(9, seq(
      field('operator', choice('!', '-')),
      field('argument', $._expression)
    )),

    binary_expression: $ => choice(
      ...[
        ['*', 7],
        ['/', 7],
        ['+', 6],
        ['-', 6],
        ['<', 5],
        ['>', 5],
        ['==', 4],
        ['!=', 4],
      ].map(([operator, precedence]) =>
        prec.left(precedence, seq(
          field('left', $._expression),
          field('operator', operator),
          field('right', $._expression)
        ))
      )
    ),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
    
    number: $ => /\d+(\.\d+)?/,
    
    string: $ => /"[^"]*"/,
    
    symbol: $ => seq(':', $.identifier),
  }
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}
