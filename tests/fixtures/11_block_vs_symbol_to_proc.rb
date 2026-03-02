numbers.map { |number| number.to_s }
numbers.any? { |number| number.even? }
numbers.find { |number| number.even? }
