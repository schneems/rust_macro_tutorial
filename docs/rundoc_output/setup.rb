require 'ostruct'
require 'erb'
BACKTICKS = "```"

TEMPLATES = Hash.new {|h, k|
    h[k] = Hash.new {|hash, key| hash[key] = []}
}
RUST_FILE = ERB.new(<<-'FILE', nil, "<>")
// File: `<%= filename %>`
<% if !TEMPLATES[filename][:module_docs].empty? %>
<%= "// Module docs ...\n" if TEMPLATES[filename][:module_docs] != module_docs %>
<%= module_docs.join("\n") %>
<% end %>
<% if !TEMPLATES[filename][:mod].empty? %>
<%= "// Mod ...\n" if TEMPLATES[filename][:mod] != mod %>
<%= mod.join("\n") %>

<% end %>
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


def write_rust_file(filename:)
  File.open(filename, "w+") do |f|
    f.write(render_rust(filename: filename, **TEMPLATES[filename]))
  end
end

def render_rust(filename: , use: nil, code: nil, test_use: nil, test_code: nil, module_docs: nil, mod: nil)
  mod = Array(mod)
  use = Array(use)
  code = Array(code)
  test_use = Array(test_use)
  test_code = Array(test_code)
  module_docs = Array(module_docs)

  RUST_FILE.result(
    OpenStruct.new(
      mod: mod,
      use: use,
      code: code,
      test_use: test_use,
      test_code: test_code
    ).instance_eval { binding }
  )
end

def replace_one(filename: , key: , match: , value: , between_text: )
  index = TEMPLATES[filename][key].find_index { |entry| entry.match?(match) }
  raise "No match for /#{match}/ in {}, expected to replace with:\n#{value}" unless index
  result = String.new
  result << "```rust"
  hash = {}
  hash[key] = TEMPLATES[filename][key][index]
  result << render_rust(filename: filename, **hash)
  result << "```"
  result << "\n\n"
  TEMPLATES[filename][key][index] = value
  result << between_text
  result << "```rust"
  hash = {}
  hash[key] = value
  result << render_rust(filename: filename, **hash)
  result << "```\n"
end

def replace(
  filename:,
  match: ,
  between_text: "With with these new contents:\n\n",
  use: nil,
  code: nil,
  test_use: nil,
  test_code: nil,
  module_docs: nil
  )
  result = String.new
  if use
    result << replace_one(filename: filename, match: match, key: :use, value: use, between_text: between_text)
  end

  if code
    result << replace_one(filename: filename, match: match, key: :code, value: code, between_text: between_text)
  end

  if test_use
    result << replace_one(filename: filename, match: match, key: :test_use, value: test_use, between_text: between_text)
  end

  if test_code
    result << replace_one(filename: filename, match: match, key: :test_code, value: test_code, between_text: between_text)
  end

  write_rust_file(filename: filename)
  result
end

def append(
  filename: ,
  mod: nil,
  use: nil,
  code: nil,
  test_use: nil,
  test_code: nil,
  module_docs: nil
  )

  added = {}
  added[:mod] = Array(mod) if mod
  added[:use] = Array(use) if use
  added[:code] = Array(code) if code
  added[:test_use] = Array(test_use) if test_use
  added[:test_code] = Array(test_code) if test_code
  added[:module_docs] = Array(module_docs) if module_docs

  added.each do |key, value|
    TEMPLATES[filename][key] += value
  end

  partial = render_rust(filename: filename, **added)
  write_rust_file(filename: filename)

  partial
end


def prepend(
  filename: ,
  use: nil,
  code: nil,
  test_use: nil,
  test_code: nil
  )

  added = {}
  added[:use] = Array(use) if use
  added[:code] = Array(code) if code
  added[:test_use] = Array(test_use) if test_use
  added[:test_code] = Array(test_code) if test_code

  added.each do |key, value|
    TEMPLATES[filename][key] = value + TEMPLATES[filename][key]
  end

  partial = render_rust(filename: filename, **added)
  write_rust_file(filename: filename)

  partial
end
