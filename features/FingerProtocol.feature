Feature: Finger protocol surface
  As a remote finger client
  I want to query the user's finger information over TCP
  So that I get a standards-compliant RFC 1288 answer on port 7979

  Background:
    Given the finger server is running on an ephemeral port
    And the user has a project file with content "Deploying thimbl"
    And the user has a plan file with content "Ship the finger server"

  Scenario: Empty query returns the full user card
    When a client connects and sends an empty query
    Then the response contains the user's login and real name
    And the response contains the project content "Deploying thimbl"
    And the response contains the plan content "Ship the finger server"
    And the server closes the connection

  Scenario: Query naming the published user returns the long format
    When a client connects and sends the user's login name
    Then the response contains the login, real name, directory and shell
    And the response contains the project content "Deploying thimbl"
    And the response contains the plan content "Ship the finger server"
    And the server closes the connection

  Scenario: Query naming the real name returns the long format
    When a client connects and sends the user's real name
    Then the response contains the user's login and real name
    And the response contains the plan content "Ship the finger server"

  Scenario: Query naming an unknown user gets the no-match answer
    When a client connects and sends the name "ghostuser"
    Then the response states that the user was not found
    And the server closes the connection

  Scenario: Forwarding query is refused
    When a client connects and sends the query "someone@elsewhere.example"
    Then the response contains "Finger forwarding service denied"
    And the server closes the connection

  Scenario: Overlong query is refused
    When a client connects and sends a query of 600 characters
    Then the response contains "query too long"

  Scenario: The verbose switch is accepted
    When a client connects and sends the query "/W"
    Then the response contains the user's login and real name

  @contract
  Scenario: The empty query response conforms to its mechanical shape
    Given the finger server is running on an ephemeral port
    When a client connects and sends an empty query
    Then the response conforms to the "finger response" schema
    And the schema file is "features/scantlings/finger-response.schema.json"
