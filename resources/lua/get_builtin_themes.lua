local rt = vim.fn.expand('$VIMRUNTIME'):gsub('\\', '/')
local seen = {}
local builtins = {}

local function collect(pattern)
    for _, p in ipairs(vim.api.nvim_get_runtime_file(pattern, true)) do
        local norm = p:gsub('\\', '/')
        if norm:find(rt, 1, true) then
            local name = vim.fn.fnamemodify(p, ':t:r')
            if not seen[name] then
                seen[name] = true
                table.insert(builtins, name)
            end
        end
    end
end

collect('colors/*.vim')
collect('colors/*.lua')

table.sort(builtins)
io.write(table.concat(builtins, ','))
