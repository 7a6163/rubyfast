ARRAY.select { |x| x > 5 }.first
ARRAY.select do |x| x > 5 end.first
ARRAY.select(&:zero?).first
