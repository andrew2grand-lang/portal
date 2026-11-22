using System.Windows;
using Microsoft.Win32;

namespace InvoiceDesk;

public partial class MainWindow : Window
{
    public MainWindow()
    {
        InitializeComponent();
        var key = Registry.CurrentUser.OpenSubKey(@"Software\InvoiceDesk");
        Title = $"InvoiceDesk · {key?.GetValue("Company") ?? "No company"}";
    }

    void OnNew(object sender, RoutedEventArgs e) => new InvoiceWindow().ShowDialog();
    void OnPrint(object sender, RoutedEventArgs e) => Services.Printing.PrintSelected(Invoices);
}
