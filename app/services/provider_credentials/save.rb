require "net/http"

module ProviderCredentials
  # Verifies the key with a cheap provider call before saving it. A rejected key is not
  # saved; a timeout or network error saves it as unverified.
  class Save
    ENDPOINTS = { "deepseek" => URI("https://api.deepseek.com/models") }.freeze
    OPEN_TIMEOUT = 3
    READ_TIMEOUT = 5
    NETWORK_ERRORS = [Timeout::Error, SocketError, SystemCallError, OpenSSL::SSL::SSLError, Net::HTTPBadResponse].freeze

    def self.call(credential, request: method(:list_models))
      new(credential, request).call
    end

    def self.list_models(provider, api_key)
      uri = ENDPOINTS.fetch(provider)
      Net::HTTP.start(uri.host, uri.port, use_ssl: true, open_timeout: OPEN_TIMEOUT, read_timeout: READ_TIMEOUT) do |http_|
        http_.get(uri.path, "Authorization" => "Bearer #{api_key}", "Accept" => "application/json").code.to_i
      end
    end

    def initialize(credential, request)
      @credential = credential
      @request = request
    end

    def call
      return false unless credential.valid?

      verify
      return reject if credential.status_invalid?

      credential.save
    end

    private
    attr_reader :credential, :request

    def verify
      code = request.call(credential.provider, credential.api_key)
      case code
      when 200 then credential.record_verification(:valid)
      when 401, 403 then credential.record_verification(:invalid, error: "http_#{code}")
      else credential.record_verification(:unverified, error: "http_#{code}")
      end
    rescue *NETWORK_ERRORS => error
      credential.record_verification(:unverified, error: error.is_a?(Timeout::Error) ? "timeout" : "network")
    end

    def reject
      credential.errors.add(:api_key, :rejected)
      false
    end
  end
end
