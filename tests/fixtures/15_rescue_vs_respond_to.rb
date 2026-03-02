begin
  foo.bar
rescue NoMethodError
  puts "oops"
end

begin
  foo.bar
rescue NoMethodError, StandardError => e
  puts e
end

begin
  foo.bar
rescue StandardError
  puts "nope"
end
