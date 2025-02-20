```
:::-- print.erb
<%
require 'ostruct'
TEMPLATES = Hash.new {|h, k|
    h[k] = Hash.new {|hash, key| hash[key] = []}
}

def render_file(filename:)
  render_rust(filename: filename, **TEMPLATES[filename])
end

def render_rust(filename: , use: nil, code: nil, test_use: nil, test_code: nil)
  use = Array(use)
  code = Array(code)
  test_use = Array(test_use)
  test_code = Array(test_code)

  result = String.new
  result << "// File: `#{filename}`\n"
  result << "// Use\n"
  result << use.join("\n")
  result << "\n"
  result << "\n"
  result << "// Code\n"
  result << code.join("\n")
  result << "\n"
  if !test_use.empty? || !test_code.empty?
    result << "#[cfg(test)]\nmod tests {\n"
    result << "    // Test use\n"
    result << test_use.join("\n")
    result << "\n"
    result << "\n"
    result << "    // Test code\n"
    result << test_code.join("\n")
    result << "\n"
    result << "}\n"
  end
  result
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

  render_rust(filename: filename, **added)
end
%>
```
