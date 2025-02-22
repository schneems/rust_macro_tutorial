require 'ostruct'
require 'erb'

puts "YOLO"

TEMPLATES = Hash.new {|h, k|
    h[k] = Hash.new {|hash, key| hash[key] = []}
}
RUST_FILE = ERB.new(<<-'FILE', nil, "<>")
// File: `<%= filename %>`
<% if !TEMPLATES[filename][:use].empty? %>
<%= "// Use ...\n" if TEMPLATES[filename][:use] != use %>
<%= use.join("\n") %>
<% end %>
<% if !TEMPLATES[filename][:code].empty? %>

// Code
<%= "// ...\n" if TEMPLATES[filename][:code] != code %>
<%= code.join("\n") %><% end %>
<% if !test_use.empty? || !test_code.empty? %>

#[cfg(test)]
mod tests {
    // Test use
<%= "    // ...\n" if TEMPLATES[filename][:test_use] != test_use %>
<%= test_use.join("\n") %>

    // Test code
<%= "    // ...\n" if TEMPLATES[filename][:test_code] != test_code %>
<%= test_code.join("\n") %>
}<% end %>
FILE

def render_file(filename:)
  render_rust(filename: filename, **TEMPLATES[filename])
end

def render_rust(filename: , use: nil, code: nil, test_use: nil, test_code: nil)
  use = Array(use)
  code = Array(code)
  test_use = Array(test_use)
  test_code = Array(test_code)

  RUST_FILE.result(
    OpenStruct.new(
      use: use,
      code: code,
      test_use: test_use,
      test_code: test_code
    ).instance_eval { binding }
  )
end

def append(
  filename: ,
  use: nil,
  code: nil,
  test_use: nil,
  test_code: nil)
  added = {}

  added[:use] = Array(use) if use
  added[:code] = Array(code) if code
  added[:test_use] = Array(test_use) if test_use
  added[:test_code] = Array(test_code) if test_code

  added.each do |key, value|
    TEMPLATES[filename][key] += value
  end

  partial = render_rust(filename: filename, **added)

  File.open(filename, "w+") do |f|
    f.write(render_rust(filename: filename, **TEMPLATES[filename]))
  end
  partial
end
