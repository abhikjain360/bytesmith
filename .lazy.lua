return {
	{
		"mrcjkb/rustaceanvim",
		config = function()
			vim.g.rustaceanvim = {
				server = {
					default_settings = {
						-- rust-analyzer language server configuration
						["rust-analyzer"] = {
							cargo = {
								noDefaultFeatures = true,
								cfgs = { "test" },
							},
							checkOnSave = {
								noDefaultFeatures = true,
							},
						},
					},
				},
			}
		end,
	},
}
