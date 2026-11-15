@{
    Name        = 'shopcraft'
    Display     = 'ShopCraft'
    AppPackage  = 'shopcraft-app'
    AppBin      = 'shopcraft-app'
    AppSmoke    = @('--smoke')
    CliPackage  = 'shopcraft-mcp'
    CliBin      = 'shopcraft'
    CliSmoke    = @('status')
    McpArgs     = @('mcp')
    McpSmoke    = @('mcp')
    Installer   = $true
    Mcpb        = $true
}
