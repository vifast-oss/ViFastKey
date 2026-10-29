import UIKit

final class ViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .systemBackground
        let stack = UIStackView()
        stack.axis = .vertical
        stack.spacing = 16
        stack.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(stack)

        let title = UILabel()
        title.text = "ViFastKey"
        title.font = .systemFont(ofSize: 34, weight: .bold)
        title.textAlignment = .center
        stack.addArrangedSubview(title)

        let body = UILabel()
        body.numberOfLines = 0
        body.font = .systemFont(ofSize: 17)
        body.text = """
        Bật bàn phím:
        1. Settings → General → Keyboard → Keyboards → Add New Keyboard
        2. Chọn ViFastKey
        3. Trong app, chạm nút 🌐 để chọn ViFastKey

        Gõ thử: tieengs → tiếng
        """
        stack.addArrangedSubview(body)

        let testField = UITextView()
        testField.font = .systemFont(ofSize: 20)
        testField.layer.borderColor = UIColor.separator.cgColor
        testField.layer.borderWidth = 1
        testField.layer.cornerRadius = 8
        testField.text = ""
        testField.heightAnchor.constraint(equalToConstant: 120).isActive = true
        stack.addArrangedSubview(testField)

        let open = UIButton(type: .system)
        open.setTitle("Mở cài đặt bàn phím", for: .normal)
        open.titleLabel?.font = .systemFont(ofSize: 20)
        open.addTarget(self, action: #selector(openSettings), for: .touchUpInside)
        stack.addArrangedSubview(open)

        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: view.safeAreaLayoutGuide.leadingAnchor, constant: 28),
            stack.trailingAnchor.constraint(equalTo: view.safeAreaLayoutGuide.trailingAnchor, constant: -28),
            stack.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor, constant: 80),
        ])
    }

    @objc private func openSettings() {
        if let url = URL(string: UIApplication.openSettingsURLString) {
            UIApplication.shared.open(url)
        }
    }
}
