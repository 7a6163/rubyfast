def call_me(&block)
  block.call
end

def call_with_args(&block)
  block.call(42)
end

def no_block_arg
  block.call
end
