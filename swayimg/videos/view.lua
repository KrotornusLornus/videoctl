local M = {}

function M.new(S, U)
  local V = {}

  function V.current_path()
    local image = swayimg.gallery.get_image()
    return image and image.path or nil
  end

  function V.has_path(path)
    for _, entry in ipairs(swayimg.imagelist.get()) do
      if entry.path == path then
        return true
      end
    end
    return false
  end

  function V.filter_args()
    if S.filter.kind == "category" then
      return "--category " .. U.q(S.filter.value)
    elseif S.filter.kind == "dir" then
      return "--dir " .. U.q(S.filter.value)
    end
    return ""
  end

  function V.filter_label()
    if S.filter.kind == "category" then
      return "categoría: " .. S.filter.value
    elseif S.filter.kind == "dir" then
      return "directorio: " .. S.filter.value
    end
    return "todos"
  end

  function V.update_title(total)
    local extra = S.music and "  [MUSIC]" or ""
    swayimg.title = "videoctl — " .. V.filter_label() .. extra .. " — " .. tostring(total) .. " vídeos"
  end

  function V.reapply_marks(keep)
    for _, entry in ipairs(swayimg.imagelist.get()) do
      local should_mark = S.selected[entry.path] == true
      if entry.mark ~= should_mark then
        swayimg.gallery.select_path(entry.path)
        swayimg.gallery.mark_image(should_mark)
      end
    end
    if keep and V.has_path(keep) then
      swayimg.gallery.select_path(keep)
    end
  end

  function V.detect_root()
    local env = os.getenv("VIDEOCTL_ROOT")
    if env and env ~= "" then
      return env
    end
    local entries = swayimg.imagelist.get() or {}
    if #entries > 0 then
      local path = entries[1].path
      local dir = path and path:match("^(.*)/[^/]+$")
      if dir and (dir == S.default_root or dir:sub(1, #S.default_root + 1) == S.default_root .. "/") then
        return dir
      end
    end
    return S.default_root
  end

  function V.reload()
    local keep = V.current_path()
    local output = U.run_silent(S.bin .. " list " .. V.filter_args())
    local paths = U.lines(output)
    local empty = #paths == 0
    if empty then
      local placeholder = U.run_silent(S.bin .. " placeholder")
      if placeholder ~= "" then
        paths[1] = placeholder
      end
    end
    swayimg.imagelist.clear()
    if #paths > 0 then
      swayimg.imagelist.add(paths)
      if keep and V.has_path(keep) then
        swayimg.gallery.select_path(keep)
      else
        swayimg.gallery.select_path(paths[1])
      end
      V.reapply_marks(keep)
    end
    V.update_title(empty and 0 or #paths)
    if empty then
      U.notify("Sin vídeos. Shift+S escanear, Shift+I importar.")
    end
  end

  function V.bootstrap()
    S.root = V.detect_root()
    if not S.filter_from_env and S.filter.kind == "dir" then
      S.filter.value = S.root
    end
    V.reload()
  end

  function V.chosen_paths()
    local paths = {}
    local seen = {}
    if #S.selected_order > 0 then
      for _, entry in ipairs(swayimg.imagelist.get()) do
        if S.selected[entry.path] and not seen[entry.path] then
          paths[#paths + 1] = entry.path
          seen[entry.path] = true
        end
      end
      for _, path in ipairs(S.selected_order) do
        if S.selected[path] and not seen[path] then
          paths[#paths + 1] = path
          seen[path] = true
        end
      end
    else
      local path = V.current_path()
      if path then
        paths[1] = path
      end
    end
    return paths
  end

  function V.placeholder_selected(paths)
    return #paths == 1 and paths[1]:match("empty%.jpg$") ~= nil
  end

  function V.thumb_args(paths)
    local parts = {}
    for _, path in ipairs(paths) do
      parts[#parts + 1] = "--thumb " .. U.q(path)
    end
    return table.concat(parts, " ")
  end

  return V
end

return M
