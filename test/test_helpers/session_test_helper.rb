module SessionTestHelper
  def sign_in_as(user, password: "password123")
    post session_path, params: { email_address: user.email_address, password: password }
  end

  def sign_out
    delete session_path
  end
end
