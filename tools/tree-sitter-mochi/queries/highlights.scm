(identifier) @variable

(const_expression name: (identifier) @constant)

(wildcard_pattern) @variable.builtin

(parameter name: (identifier) @variable.parameter)
(function_type_parameter (identifier) @variable.parameter)

(member_expression property: (identifier) @variable.other.member)
(hash_pair key: (identifier) @variable.other.member)
(hash_type_field key: (identifier) @variable.other.member)

(function_definition name: (identifier) @function)

(call_expression function: (identifier) @function)
(call_expression function: (member_expression property: (identifier) @function))

(method_call method: (identifier) @function.method)

[
  "let"
  "const"
] @keyword.storage

[
  "fn"
] @keyword.function

[
  "if"
  "else"
  "do"
  "end"
  "loop"
  "while"
  "for"
  "in"
  "match"
  "when"
  "return"
  "break"
  "continue"
  "import"
] @keyword.control

(primitive_type) @type.builtin
(custom_type) @type

(number) @number
(string) @string
(atom (identifier) @string.special.symbol)
(comment) @comment

[
  "+"
  "-"
  "*"
  "/"
  "=="
  "!="
  ">"
  "<"
  "="
  "?"
  "!"
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

[
  ","
  "."
  ":"
  "|"
] @punctuation.delimiter
(atom ":" @string.special.symbol)
