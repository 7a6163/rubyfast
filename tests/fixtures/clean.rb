# A clean Ruby file with no performance offenses
class User
  attr_reader :name
  attr_writer :email

  def initialize(name, email)
    @name = name
    @email = email
  end

  def greet
    "Hello, #{name}"
  end
end

[1, 2, 3].each { |n| puts n }
[1, 2, 3].sample
[1, 2, 3].detect { |n| n > 1 }
[1, 2, 3].reverse_each { |n| puts n }
{ a: 1 }.each_key { |k| puts k }
[1, 2, 3].flat_map { |n| [n, n] }
"hello".tr("h", "H")
[1, 2, 3].sort_by { |a, b| a <=> b }
{ a: 1 }.fetch(:a) { 0 }
(1..10).cover?(5)
