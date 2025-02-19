#include "kmessage_widget.h"
#include <QMessageBox>
#include <QIcon>

extern "C" void kmessage_widget_close(QWidget* widget) {
    KMessageWidget* kmessagewidget = dynamic_cast<KMessageWidget*>(widget);
    kmessagewidget->setWordWrap(true);
    kmessagewidget->hide();
}

extern "C" void kmessage_widget_set_error(QWidget* widget, QString const text) {
    KMessageWidget* kmessagewidget = dynamic_cast<KMessageWidget*>(widget);
    kmessagewidget->hide();
    kmessagewidget->setText(text);
    kmessagewidget->setMessageType(KMessageWidget::MessageType::Error);
    kmessagewidget->setIcon(QIcon::fromTheme("dialog-error"));
    kmessagewidget->animatedShow();
}

extern "C" void kmessage_widget_set_warning(QWidget* widget, QString const text) {
    KMessageWidget* kmessagewidget = dynamic_cast<KMessageWidget*>(widget);
    kmessagewidget->hide();
    kmessagewidget->setText(text);
    kmessagewidget->setMessageType(KMessageWidget::MessageType::Warning);
    kmessagewidget->setIcon(QIcon::fromTheme("dialog-warning"));
    kmessagewidget->animatedShow();
}

extern "C" void kmessage_widget_set_info(QWidget* widget, QString const text) {
    KMessageWidget* kmessagewidget = dynamic_cast<KMessageWidget*>(widget);
    kmessagewidget->hide();
    kmessagewidget->setText(text);
    kmessagewidget->setMessageType(KMessageWidget::MessageType::Information);
    kmessagewidget->setIcon(QIcon::fromTheme("dialog-information"));
    kmessagewidget->animatedShow();
}
