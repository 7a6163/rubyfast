# frozen_string_literal: true

# Frozen plain literals cost nothing to construct.
ENV.fetch("PORT", "3000")
h.fetch(:k, "")

# Interpolation and collections still allocate on every call.
h.fetch(:k, "sum+#{name}")
h.fetch(:k, [])
