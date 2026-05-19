[
  "if"
  "else"
  "do"
  "end"
  "fn"
  "return"
  "let"
  "const"
] @keyword

(number) @number
(string) @string
(boolean) @boolean

(comment) @comment

[
  "+" "-" "*" "/" 
  "==" "!=" "<" ">" 
  "=" "!"
] @operator

[
  "(" ")"
  "{" "}"
  "[" "]"
] @punctuation.bracket

[
  "," "." ":"
] @punctuation.delimiter

(function_expression name: (identifier) @function)

(call_expression function: (identifier) @function.call)

(let_expression name: (identifier) @variable)
(const_expression name: (identifier) @variable)

(dot_expression property: (identifier) @property)

(hash_pair key: (identifier) @property)

(identifier) @variable
