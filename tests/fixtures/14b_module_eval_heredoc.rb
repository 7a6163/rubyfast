# Module eval with heredoc containing def — should fire
klass.module_eval(<<~RUBY)
  def hello
    "world"
  end
RUBY
