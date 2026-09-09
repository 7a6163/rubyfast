HASH.fetch(:writing, [*1..100])

# Cheap defaults cost nothing to construct — the block form would be slower.
ENV.fetch("TOKEN", nil)
h.fetch(:k, 0)
h.fetch(:k, :missing)
h.fetch(:k, true)
h.fetch(:k, DEFAULT)
h.fetch(:k, Foo::DEFAULT)
h.fetch(:k, @default)
h.fetch(:k, $default)
