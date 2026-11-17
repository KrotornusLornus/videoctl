local M = {}

function M.new(S, U, F, V)
  local C = {}

  function C.scan()
    local dirs = U.lines(U.run_silent(S.bin .. " scan-dirs"))
    local selected_dir = F.menu(dirs, "Escanear> ", "Filtrar o escribir carpeta")
    if not selected_dir then
      U.notify("Escaneo cancelado.")
      return
    end
    local output = U.run_action(S.bin .. " scan --dir " .. U.q(selected_dir))
    U.notify(output ~= "" and output or "Escaneo terminado.")
    V.reload()
  end

  function C.import_urls()
    local path = os.tmpname()
    local file = io.open(path, "w")
    if not file then
      U.notify("No se pudo crear el fichero de importación.")
      return
    end
    file:write("# Una URL por línea. Las líneas que empiezan por # se ignoran.\n")
    file:write("# No se soportan playlists por ahora.\n\n")
    file:close()
    local editor = os.getenv("EDITOR") or os.getenv("VISUAL") or "vi"
    local command = "foot -e sh -c " .. U.q(editor .. " " .. U.q(path))
    local ok = os.execute(command)
    if not ok then
      os.remove(path)
      U.notify("Importación cancelada.")
      return
    end
    local output = U.run_action(S.bin .. " import-file " .. U.q(path))
    os.remove(path)
    U.notify(output ~= "" and output or "Importación terminada.")
    V.reload()
  end

  function C.pick_category()
    local entries = { "Todas" }
    for _, path in ipairs(U.lines(U.run_silent(S.bin .. " categories"))) do
      entries[#entries + 1] = path
    end
    local value = F.menu(entries, "Categoría> ", "Filtrar categoría")
    if not value then
      return
    end
    if value == "Todas" then
      S.filter.kind = "all"
      S.filter.value = ""
    else
      S.filter.kind = "category"
      S.filter.value = value:gsub("^/", ""):gsub("/$", "")
    end
    V.reload()
  end

  function C.pick_dir()
    local entries = { "Todas" }
    for _, path in ipairs(U.lines(U.run_silent(S.bin .. " dirs"))) do
      entries[#entries + 1] = path
    end
    local value = F.menu(entries, "Directorio> ", "Filtrar o escribir ruta")
    if not value then
      return
    end
    if value == "Todas" then
      S.filter.kind = "all"
      S.filter.value = ""
    else
      S.filter.kind = "dir"
      S.filter.value = value
    end
    V.reload()
  end

  function C.assign_category()
    local paths = V.chosen_paths()
    if #paths == 0 or V.placeholder_selected(paths) then
      U.notify("No hay vídeo seleccionado.")
      return
    end
    local entries = U.lines(U.run_silent(S.bin .. " categories"))
    entries[#entries + 1] = "Nueva categoría…"
    local category = F.menu(entries, "Asignar> ", "Elegir o crear categoría")
    if not category then
      return
    end
    local create = false
    if category == "Nueva categoría…" then
      category = F.text("Nueva categoría> ", "A/B/C")
      if not category then
        return
      end
      create = true
    end
    local command = S.bin .. " assign" .. (create and " --create" or "")
      .. " --category " .. U.q(category) .. " " .. V.thumb_args(paths)
    local output = U.run_action(command)
    U.notify(output ~= "" and output or "Categoría asignada.")
    V.reload()
  end

  function C.manage_categories()
    local messages = {}
    while true do
      local action = F.menu(
        { "Crear", "Renombrar", "Mover", "Borrar", "Salir" },
        "Categorías> ",
        "Elegir acción"
      )
      if not action or action == "Salir" then
        break
      end
      local categories = U.lines(U.run_silent(S.bin .. " categories --no-system"))
      local output = ""
      if action == "Crear" then
        local path = F.text("Nueva categoría> ", "A/B/C")
        if path then
          output = U.run_action(S.bin .. " category create --path " .. U.q(path))
        end
      elseif #categories > 0 then
        if action == "Renombrar" then
          local path = F.menu(categories, "Renombrar> ", "Elegir categoría")
          if path then
            local current = path:match("([^/]+)$") or path
            local name = F.text("Nuevo nombre> ", current, current)
            if name then
              output = U.run_action(
                S.bin .. " category rename --path " .. U.q(path) .. " --name " .. U.q(name)
              )
            end
          end
        elseif action == "Mover" then
          local path = F.menu(categories, "Mover> ", "Elegir categoría")
          if path then
            local parents = { "Raíz" }
            for _, value in ipairs(categories) do
              parents[#parents + 1] = value
            end
            local parent = F.menu(parents, "Nuevo padre> ", "Elegir destino")
            if parent then
              local command = S.bin .. " category move --path " .. U.q(path)
              if parent ~= "Raíz" then
                command = command .. " --parent " .. U.q(parent)
              end
              output = U.run_action(command)
            end
          end
        elseif action == "Borrar" then
          local path = F.menu(categories, "Borrar> ", "Elegir categoría")
          if path then
            local confirm = F.menu(
              { "Cancelar", "Borrar categoría y subcategorías" },
              "Confirmar> ",
              "Elegir acción"
            )
            if confirm == "Borrar categoría y subcategorías" then
              output = U.run_action(S.bin .. " category delete --path " .. U.q(path))
            end
          end
        end
      end
      if output ~= "" then
        messages[#messages + 1] = output
      end
    end
    U.notify(#messages > 0 and table.concat(messages, "\n") or "Sin cambios en categorías.")
    V.reload()
  end

  return C
end

return M
