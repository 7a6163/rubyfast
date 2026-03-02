h.merge!(item: 1)
h.merge!({item: 1})
ENUM.each_with_object({}) do |e, h|
  h.merge!(e => e)
end
