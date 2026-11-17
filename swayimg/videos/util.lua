local M = {}

function M.new(state)
  local U = {}

  function U.q(value)
    return "'" .. tostring(value):gsub("'", "'\\''") .. "'"
  end

  function U.trim(value)
    return (value or ""):gsub("^%s+", ""):gsub("%s+$", "")
  end

  function U.run(command)
    local pipe = io.popen(command, "r")
    if not pipe then
      return ""
    end
    local output = pipe:read("*a")
    pipe:close()
    return U.trim(output)
  end

  function U.run_silent(command)
    return U.run(command .. " 2>/dev/null")
  end

  function U.run_action(command)
    return U.run(command .. " 2>&1")
  end

  function U.lines(output)
    local result = {}
    for line in (output or ""):gmatch("[^\r\n]+") do
      local value = U.trim(line)
      if value ~= "" then
        result[#result + 1] = value
      end
    end
    return result
  end

  function U.notify(message)
    state.help_visible = false
    swayimg.text.visible = true
    swayimg.text.status = message
  end

  return U
end

return M
