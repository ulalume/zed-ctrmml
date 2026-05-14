; Token-level bracket pairs. The grammar emits `[`, `]`, `{`, `}`, `(`, `)`
; as `punctuation`, and string/quote tokens as `string`. Zed pairs them by
; matching the captures inside this query, so the editor's bracket-jump
; and surround commands work even though the grammar doesn't model them
; as nested constructs.
("[" @open "]" @close)
("{" @open "}" @close)
("(" @open ")" @close)
