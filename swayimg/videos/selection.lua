local M = {}

function M.new(S, U, V)
  local A = {}

  function A.remove_selected(path)
    S.selected[path] = nil
    for index, value in ipairs(S.selected_order) do
      if value == path then
        table.remove(S.selected_order, index)
        break
      end
    end
  end

  function A.toggle_mark()
    local path = V.current_path()
    if not path then
      return
    end
    local state = not (S.selected[path] == true)
    if state then
      S.selected[path] = true
      S.selected_order[#S.selected_order + 1] = path
    else
      A.remove_selected(path)
    end
    swayimg.gallery.mark_image(state)
    U.notify("Marcados: " .. tostring(#S.selected_order))
  end

  function A.clear_marks()
    local keep = V.current_path()
    for _, entry in ipairs(swayimg.imagelist.get()) do
      if entry.mark then
        swayimg.gallery.select_path(entry.path)
        swayimg.gallery.mark_image(false)
      end
    end
    S.selected = {}
    S.selected_order = {}
    if keep and V.has_path(keep) then
      swayimg.gallery.select_path(keep)
    end
    U.notify("Selección limpiada.")
  end

  function A.remove_from_view()
    local path = V.current_path()
    if not path then
      return
    end
    A.remove_selected(path)
    swayimg.imagelist.remove(path)
    U.notify("Quitado del visor (sin borrar metadata).")
    if swayimg.imagelist.size == 0 then
      V.reload()
    end
  end

  return A
end

return M
