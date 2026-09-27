Feature: Server robustness
  As the single user of the finger server
  I want the server to stay responsive under hostile connection loads
  So that a swarm of idle clients cannot take the service down

  Background:
    Given the finger server is running on an ephemeral port

  Scenario: Idle connections are reaped after a timeout
    Given the read timeout of the server is 10 seconds
    When a client connects and sends nothing
    Then the connection is closed by the server within 15 seconds

  Scenario: A slow client cannot block other queries
    Given a client connected and sent a partial query
    When another client queries the user's login name
    Then the other client receives the full card

  Scenario: The accept loop survives file descriptor exhaustion
    Given the server process has a file descriptor limit of 64
    When 70 clients connect and stay silent
    And a new client queries the user's login name
    Then the new client receives a response or a refusal within 2 seconds

  Scenario: The server logs one line per query
    When a client connects and sends the query "someone@elsewhere.example"
    Then the server output contains a line naming the refused query
    And the server output contains the client address
