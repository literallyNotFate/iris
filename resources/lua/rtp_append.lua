local p = vim.fn.stdpath('data') .. '/{}'
for _, dir in ipairs(vim.fn.expand(p .. '/*', false, true)) do
    vim.opt.rtp:append(dir)
end
