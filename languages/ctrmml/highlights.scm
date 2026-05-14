(comment) @comment
((platform_command_keyword) @keyword
  (#set! "priority" 110))
(string) @string
(number) @number

; Meta keyword names (`#title`, `#platform`, ...). The grammar emits a
; dedicated node per discriminated keyword; they all share @preproc.
(meta_keyword) @preproc
(platform_meta_keyword) @preproc
(option_meta_keyword) @preproc
(group_meta_keyword) @preproc
(timesig_meta_keyword) @preproc

; Known meta values (the meaningful keywords for each meta type) pop as
; @keyword next to their preprocessor sibling. The generic `meta_value`
; fallback captures arbitrary text (song titles, comments, etc.).
(platform_known_value) @keyword
(option_known_value) @keyword
(group_known_value) @keyword
(timesig_known_value) @keyword
(meta_value) @string

(at_command) @function
(track_selector) @title

(instrument_type) @type
(note) @constant
(rest) @constant

(command_with_number) @keyword
(command) @keyword
(escape_command) @keyword
(key_signature) @keyword

(operator) @operator
(punctuation) @punctuation.delimiter
(param_key) @property
