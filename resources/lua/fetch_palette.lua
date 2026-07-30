local function normalize(hex)
    if type(hex) == 'string' then
        return hex:lower()
    end
    return hex
end

local function g(name, attr)
    local max_depth = 15
    local current = name
    local visited = {}

    for _ = 1, max_depth do
        if visited[current] then break end
        visited[current] = true

        local hl = vim.api.nvim_get_hl(0, { name = current, link = false })

        local color = nil
        if attr == 'bg' then
            color = hl.bg
        else
            color = hl.fg
        end

        if color ~= nil then
            return string.format('#%06x', color)
        end

        local linked = vim.api.nvim_get_hl(0, { name = current, link = true })
        if not linked.link or linked.link == current then
            break
        end
        current = linked.link
    end

    return nil
end

local function first(attr, names)
    for _, name in ipairs(names) do
        local c = g(name, attr)
        if c then return c end
    end
    return nil
end

local function chain(attr, ...)
    for _, names in ipairs({...}) do
        local c = first(attr, names)
        if c then return c end
    end
    return '#cccccc'
end

local fg = g('Normal', 'fg') or '#cccccc'
local bg = g('Normal', 'bg') or '#1c1c1c'

local p_red     = chain('fg', { 'DiagnosticError', 'ErrorMsg' },    { 'DiffDelete' })
local p_green   = chain('fg', { 'DiagnosticOk', 'DiagnosticHint' }, { 'String', '@string' })
local p_yellow  = chain('fg', { 'DiagnosticWarn', 'WarningMsg' },   { 'Number', '@number' })
local p_blue    = chain('fg', { 'Function', '@function' },         { 'Directory' })
local p_magenta = chain('fg', { 'Keyword', '@keyword' },            { 'Special' })
local p_cyan    = chain('fg', { 'Type', '@type' },                  { 'Identifier' })
local p_dim     = chain('fg', { 'Comment', '@comment' },            { 'NonText' })

local semantic_fallback = {
    bg,
    p_red,
    p_green,
    p_yellow,
    p_blue,
    p_magenta,
    p_cyan,
    p_dim,
    p_dim,
    p_red,
    p_green,
    p_yellow,
    p_blue,
    p_magenta,
    p_cyan,
    fg,
}

local function resolve_ansi()
    local result = {}
    local has_any = false

    for i = 0, 15 do
        if vim.g['terminal_color_' .. i] ~= nil then
            has_any = true
            break
        end
    end

    if has_any then
        for i = 0, 15 do
            local color = vim.g['terminal_color_' .. i]
            if type(color) == 'string' then
                table.insert(result, normalize(color))
            elseif type(color) == 'number' then
                table.insert(result, string.format('#%06x', color))
            else
                table.insert(result, normalize(semantic_fallback[i + 1]))
            end
        end
    else
        for i = 1, 16 do
            table.insert(result, normalize(semantic_fallback[i]))
        end
    end
    return result
end

local white = '#ffffff'
if vim.g.terminal_color_15 ~= nil then
    if type(vim.g.terminal_color_15) == 'string' then
        white = vim.g.terminal_color_15
    elseif type(vim.g.terminal_color_15) == 'number' then
        white = string.format('#%06x', vim.g.terminal_color_15)
    end
else
    white = fg
end

local res = {
    bg        = bg,
    fg        = fg,

    caret     = chain('bg',
        { 'Cursor', 'TermCursor' },
        { 'CursorLine' }
    ),

    line_hl   = chain('bg',
        { 'CursorLine', 'CursorLineBg' },
        { 'ColorColumn' }
    ),

    sel       = chain('bg',
        { 'Visual', 'Selection', 'PmenuSel' }
    ),

    gutter_fg = chain('fg',
        { 'LineNr', 'SignColumn', 'FoldColumn' },
        { 'Comment' }
    ),

    comment   = chain('fg',
        { 'Comment', '@comment', '@comment.line', '@comment.block' }
    ),

    variable  = chain('fg',
        { '@variable', '@variable.member', '@variable.parameter' },
        { 'Identifier' }
    ),

    constant  = chain('fg',
        { '@constant', '@constant.builtin', '@constant.macro' },
        { 'Constant', 'Special' },
        { '@number', 'Number' }
    ),

    number    = chain('fg',
        { '@number', '@number.float', '@number.integer' },
        { 'Number', 'Float' },
        { 'Constant' }
    ),

    string    = chain('fg',
        { '@string', '@string.special', '@string.escape' },
        { 'String', 'Character' }
    ),

    keyword   = chain('fg',
        { '@keyword', '@keyword.function', '@keyword.import', '@keyword.return' },
        { 'Keyword', 'Statement', 'Conditional', 'Repeat' }
    ),

    operator  = chain('fg',
        { '@operator', '@keyword.operator' },
        { 'Operator' }
    ),

    func      = chain('fg',
        { '@function', '@function.call', '@function.builtin', '@function.method', '@function.method.call' },
        { 'Function' }
    ),

    type_name = chain('fg',
        { '@type', '@type.builtin', '@type.definition' },
        { 'Type', 'Typedef' }
    ),

    tag       = chain('fg',
        { '@tag', '@tag.builtin' },
        { 'Tag', 'Special' }
    ),

    attribute = chain('fg',
        { '@attribute', '@property', '@tag.attribute' },
        { 'Identifier' }
    ),

    added     = chain('fg',
        { 'DiffAdd', 'GitSignsAdd', '@diff.plus', 'Added' }
    ),

    deleted   = chain('fg',
        { 'DiffDelete', 'GitSignsDelete', '@diff.minus', 'Removed' }
    ),

    changed   = chain('fg',
        { 'DiffChange', 'GitSignsChange', '@diff.delta', 'Changed' }
    ),

    white     = normalize(white),
    ansi      = resolve_ansi(),
}

for k, v in pairs(res) do
    if type(v) == 'string' then
        res[k] = normalize(v)
    end
end

io.write(vim.fn.json_encode(res))
