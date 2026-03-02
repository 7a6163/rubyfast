# This file tests inline disable comments.

# Trailing disable — should NOT be reported
x = [].shuffle.first # rubyfast:disable shuffle_first_vs_sample

# No disable — should be reported
y = [].shuffle.first

# disable-next-line — next line should NOT be reported
# rubyfast:disable-next-line shuffle_first_vs_sample
z = [].shuffle.first

# But this one should be reported
w = [].shuffle.first

# Block disable — should NOT be reported
# rubyfast:disable for_loop_vs_each
for i in [1, 2, 3]
  puts i
end
# rubyfast:enable for_loop_vs_each

# After enable — should be reported
for j in [4, 5, 6]
  puts j
end

# Fasterer compat — should NOT be reported
q = [].shuffle.first # fasterer:disable shuffle_first_vs_sample
