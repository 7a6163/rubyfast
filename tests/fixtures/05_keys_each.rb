HASH.keys.each { |k| puts k }
HASH.keys.each(&:to_sym)
HASH.keys.each do |k| puts k end
