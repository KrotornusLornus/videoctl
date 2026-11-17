local M = {}

function M.new(U)
  local F = {}

  function F.menu(entries, prompt, placeholder)
    if #entries == 0 then
      return nil
    end
    local path = os.tmpname()
    local file = io.open(path, "w")
    if not file then
      return nil
    end
    for _, entry in ipairs(entries) do
      file:write(tostring(entry):gsub("[\r\n]+", " "), "\n")
    end
    file:close()
    local command = "fuzzel --dmenu --prompt " .. U.q(prompt)
      .. " --placeholder " .. U.q(placeholder)
      .. " --lines 20 --width 70 < " .. U.q(path) .. " 2>/dev/null"
    local output = U.run(command)
    os.remove(path)
    if output == "" then
      return nil
    end
    return output
  end

  function F.text(prompt, placeholder, initial)
    local command = "fuzzel --dmenu --prompt-only --prompt " .. U.q(prompt)
      .. " --placeholder " .. U.q(placeholder)
    if initial and initial ~= "" then
      command = command .. " --search " .. U.q(initial)
    end
    command = command .. " </dev/null 2>/dev/null"
    local output = U.run(command)
    if output == "" then
      return nil
    end
    return output
  end

  return F
end

return M
